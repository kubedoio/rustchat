//! File upload validation helpers

use crate::constants::*;
use crate::error::AppError;

pub const ALLOWED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "pdf", "txt", "md", "zip",
];

/// Validate only that a filename has an allowed extension, returning the lowercase extension.
pub fn validate_file_extension(filename: &str) -> Result<String, AppError> {
    let ext = filename
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();

    if ext.is_empty() {
        return Err(AppError::BadRequest(
            "File must have an allowed extension".to_string(),
        ));
    }

    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::BadRequest(format!(
            "File extension '.{}' is not allowed",
            ext
        )));
    }

    Ok(ext)
}

/// Validate a file upload and return the canonical MIME type and lowercase extension.
pub fn validate_file_upload(filename: &str, data: &[u8]) -> Result<(String, String), AppError> {
    let ext = validate_file_extension(filename)?;

    let max_size = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => MAX_IMAGE_SIZE,
        "pdf" | "txt" | "md" => MAX_DOCUMENT_SIZE,
        _ => MAX_OTHER_FILE_SIZE,
    };

    if data.len() > max_size {
        return Err(AppError::BadRequest(format!(
            "File exceeds maximum size of {} bytes for this type",
            max_size
        )));
    }

    let expected_mime = extension_to_mime(&ext);
    let actual_mime = detect_mime_from_bytes(data);

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp"
            if actual_mime.as_deref() != Some(expected_mime) =>
        {
            Err(AppError::BadRequest(format!(
                "File content does not match extension '.{}'. Expected {}, got {}",
                ext,
                expected_mime,
                actual_mime.as_deref().unwrap_or("unknown")
            )))
        }
        "pdf" if actual_mime.as_deref() != Some("application/pdf") => Err(AppError::BadRequest(
            "File content does not match declared PDF extension".to_string(),
        )),
        "zip" if actual_mime.as_deref() != Some("application/zip") => Err(AppError::BadRequest(
            "File content does not match declared ZIP extension".to_string(),
        )),
        "txt" | "md" => {
            if std::str::from_utf8(data).is_err() {
                return Err(AppError::BadRequest(
                    "Text files must be valid UTF-8".to_string(),
                ));
            }
            // Prevent binary files masquerading as text
            if actual_mime.is_some() {
                return Err(AppError::BadRequest(
                    "File content does not match declared text extension".to_string(),
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }?;

    Ok((expected_mime.to_string(), ext))
}

/// Validate raw image bytes (for emoji uploads without a filename).
/// Returns the canonical MIME type.
pub fn validate_image_bytes(data: &[u8]) -> Result<String, AppError> {
    if data.len() > MAX_IMAGE_SIZE {
        return Err(AppError::BadRequest(format!(
            "Image exceeds maximum size of {} bytes",
            MAX_IMAGE_SIZE
        )));
    }

    let mime = detect_mime_from_bytes(data)
        .ok_or_else(|| AppError::BadRequest("Could not determine image format".to_string()))?;

    match mime.as_str() {
        "image/png" | "image/jpeg" | "image/gif" | "image/webp" => Ok(mime),
        _ => Err(AppError::BadRequest(
            "Invalid image format. Only PNG, JPEG, GIF and WEBP are allowed".to_string(),
        )),
    }
}

fn extension_to_mime(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

fn detect_mime_from_bytes(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }

    // PNG
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Some("image/png".to_string());
    }

    // JPEG
    if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        return Some("image/jpeg".to_string());
    }

    // GIF
    if data.len() >= 6 && (data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) {
        return Some("image/gif".to_string());
    }

    // WEBP
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some("image/webp".to_string());
    }

    // PDF
    if data.starts_with(b"%PDF") {
        return Some("application/pdf".to_string());
    }

    // ZIP
    if data.starts_with(b"PK") && data.len() >= 4 {
        let sig = (data[2], data[3]);
        if matches!(sig, (0x03, 0x04) | (0x05, 0x06) | (0x07, 0x08)) {
            return Some("application/zip".to_string());
        }
    }

    None
}

/// Validate a file upload using only the first bytes and total size.
/// For SVG and text files, additional full-content validation must be performed by the caller.
pub fn validate_file_upload_head(
    filename: &str,
    head: &[u8],
    size: usize,
) -> Result<(String, String), AppError> {
    let ext = validate_file_extension(filename)?;

    let max_size = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => MAX_IMAGE_SIZE,
        "pdf" | "txt" | "md" => MAX_DOCUMENT_SIZE,
        _ => MAX_OTHER_FILE_SIZE,
    };

    if size > max_size {
        return Err(AppError::BadRequest(format!(
            "File exceeds maximum size of {} bytes for this type",
            max_size
        )));
    }

    let expected_mime = extension_to_mime(&ext);
    let actual_mime = detect_mime_from_bytes(head);

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp"
            if actual_mime.as_deref() != Some(expected_mime) =>
        {
            Err(AppError::BadRequest(format!(
                "File content does not match extension '.{}'. Expected {}, got {}",
                ext,
                expected_mime,
                actual_mime.as_deref().unwrap_or("unknown")
            )))
        }
        "pdf" if actual_mime.as_deref() != Some("application/pdf") => Err(AppError::BadRequest(
            "File content does not match declared PDF extension".to_string(),
        )),
        "zip" if actual_mime.as_deref() != Some("application/zip") => Err(AppError::BadRequest(
            "File content does not match declared ZIP extension".to_string(),
        )),
        _ => Ok(()),
    }?;

    Ok((expected_mime.to_string(), ext))
}

#[allow(dead_code)]
/// Raw-text screening of SVG content (defense-in-depth; SVG uploads are
/// rejected at the extension layer today, so this validator is not on a
/// live path).
///
/// Known limitations, accepted for a screening layer:
///
/// - **Entity encoding**: an XML/HTML parser decodes entities
///   (`&#106;avascript:`, `on&#108;oad=`), so entity-encoded payloads can
///   pass raw-text screening. If this validator is ever wired to a live
///   path, replace or augment it with a real XML parse plus
///   allowlist-based sanitization — do not rely on substring/regex
///   screening alone against an XML-parsing consumer.
/// - **Content false positives**: patterns are matched over the whole
///   document, so handler-like text inside `<text>` nodes or inert CDATA
///   is also rejected. Conservative by intent.
fn validate_svg(data: &[u8]) -> Result<(), AppError> {
    use regex::Regex;
    use std::sync::LazyLock;

    /// Any `on*= ` event-handler attribute (onclick, onbegin, onanimationstart,
    /// whitespace variants like `onload =`). An allowlist of specific handler
    /// names is trivially bypassed by lesser-known or future event names.
    static EVENT_HANDLER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)\bon[a-z]+\s*="#).unwrap());
    /// Active-content URI schemes anywhere in the document (SMIL `values=`,
    /// `<style>` url(), attribute values): `javascript:` / `vbscript:` payloads
    /// and `data:` URIs that can carry nested active documents.
    static ACTIVE_URI_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(javascript|vbscript)\s*:|data\s*:\s*(text/html|image/svg\+xml)").unwrap()
    });
    /// SMIL animation that rewrites the `href`/`xlink:href` attribute at runtime
    /// (bypasses the static `href=` screening below).
    static SMIL_HREF_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)attributename\s*=\s*["'](href|xlink:href)["']"#).unwrap()
    });
    /// CSS that fetches remote resources from a `style` attribute. Local
    /// fragment references (`url(#gradient)`, `url(#pattern)`) are explicitly
    /// allowed — they are the standard way to reference in-document
    /// gradients/patterns and cannot fetch anything.
    static STYLE_FETCH_RE: LazyLock<Regex> = LazyLock::new(|| {
        // `url(` must be followed by an optional quote and then something
        // other than `#` (fragment), a quote, whitespace, or `)` (empty).
        Regex::new(r#"(?i)style\s*=\s*["'][^"']*\b(url|expression)\s*\(\s*["']?[^#"'\s)]"#).unwrap()
    });
    /// Any `href`/`xlink:href` attribute, with arbitrary whitespace around
    /// `=` (`href = "…"` must not slip past the screen the way a plain
    /// `contains("href=")` check allowed).
    static HREF_ATTR_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)(?:^|[\s<"'(])(xlink:)?href\s*="#).unwrap());

    let text = std::str::from_utf8(data)
        .map_err(|_| AppError::BadRequest("SVG must be valid UTF-8".to_string()))?;

    let trimmed = text.trim_start();
    if !trimmed.starts_with("<?xml")
        && !trimmed.starts_with("<svg")
        && !trimmed.starts_with("<!DOCTYPE")
    {
        return Err(AppError::BadRequest(
            "SVG does not have a valid XML preamble".to_string(),
        ));
    }

    let lower = text.to_ascii_lowercase();

    // Reject script tags
    if lower.contains("<script") || lower.contains("</script>") {
        return Err(AppError::BadRequest(
            "SVG contains forbidden script elements".to_string(),
        ));
    }

    // Reject event handlers (any on* attribute, not a fixed allowlist)
    if EVENT_HANDLER_RE.is_match(text) {
        return Err(AppError::BadRequest(
            "SVG contains forbidden event handler attributes".to_string(),
        ));
    }

    // Reject foreignObject (can embed HTML)
    if lower.contains("<foreignobject") || lower.contains("</foreignobject>") {
        return Err(AppError::BadRequest(
            "SVG contains forbidden foreignObject elements".to_string(),
        ));
    }

    // Reject external references (href/xlink:href attributes, whitespace
    // around `=` included)
    if HREF_ATTR_RE.is_match(text) {
        return Err(AppError::BadRequest(
            "SVG contains forbidden external references".to_string(),
        ));
    }

    // Reject active-content URI schemes anywhere (javascript:, vbscript:,
    // data:text/html, data:image/svg+xml) — e.g. inside SMIL animation values.
    if ACTIVE_URI_RE.is_match(text) {
        return Err(AppError::BadRequest(
            "SVG contains forbidden active-content URI schemes".to_string(),
        ));
    }

    // Reject <style> elements (CSS-based exfiltration / CSP interaction)
    if lower.contains("<style") || lower.contains("</style>") {
        return Err(AppError::BadRequest(
            "SVG contains forbidden style elements".to_string(),
        ));
    }

    // Reject style attributes that fetch remote resources
    if STYLE_FETCH_RE.is_match(text) {
        return Err(AppError::BadRequest(
            "SVG contains a style attribute with a forbidden fetch".to_string(),
        ));
    }

    // Reject SMIL animations that rewrite href/xlink:href at runtime
    if SMIL_HREF_RE.is_match(text) {
        return Err(AppError::BadRequest(
            "SVG contains forbidden SMIL href animation".to_string(),
        ));
    }

    // Reject entity declarations (XXE / billion-laughs expansion). The XML
    // preamble check above still admits plain <!DOCTYPE svg ...> documents.
    if lower.contains("<!entity") {
        return Err(AppError::BadRequest(
            "SVG contains forbidden entity declarations".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_extension_is_rejected() {
        let result = validate_file_upload("test.svg", b"<svg></svg>");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not allowed") || err.contains("svg"),
            "Expected SVG rejection, got: {}",
            err
        );
    }

    #[test]
    fn malicious_svg_with_onload_is_rejected() {
        let svg = b"<svg onload='alert(1)'></svg>";
        let result = validate_svg(svg);
        assert!(result.is_err());
    }

    #[test]
    fn malicious_svg_with_foreign_object_is_rejected() {
        let svg = b"<svg><foreignObject><script>alert(1)</script></foreignObject></svg>";
        let result = validate_svg(svg);
        assert!(result.is_err());
    }

    #[test]
    fn malicious_svg_with_external_ref_is_rejected() {
        let svg = b"<svg><use xlink:href='http://evil.com/payload.svg'/></svg>";
        let result = validate_svg(svg);
        assert!(result.is_err());
    }

    #[test]
    fn benign_svg_is_accepted_by_validator() {
        let svg = b"<svg xmlns='http://www.w3.org/2000/svg'><circle r='10'/></svg>";
        let result = validate_svg(svg);
        assert!(result.is_ok());
    }

    #[test]
    fn benign_svg_with_smil_and_style_attribute_is_accepted_by_validator() {
        // Common icon patterns must keep passing: benign SMIL animation and
        // inline style attributes without remote fetches.
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <rect style="fill:#00ff00" width="10" height="10">
                <animate attributeName="opacity" values="0;1" dur="2s"/>
            </rect>
        </svg>"#;
        let result = validate_svg(svg);
        assert!(result.is_ok());
    }

    #[test]
    fn malicious_svg_with_uncommon_event_handler_is_rejected() {
        // The old allowlist only knew onload/onerror/onclick/onmouseover/
        // onfocus/onblur; these bypassed it.
        let cases: [&[u8]; 4] = [
            b"<svg><rect onanimationstart='alert(1)'/></svg>",
            b"<svg><rect onbegin='alert(1)'/></svg>",
            b"<svg><rect onpointerenter='alert(1)'/></svg>",
            b"<svg onload = 'alert(1)'></svg>", // whitespace variant
        ];
        for svg in cases {
            assert!(
                validate_svg(svg).is_err(),
                "expected rejection of: {}",
                String::from_utf8_lossy(svg)
            );
        }
    }

    #[test]
    fn malicious_svg_with_javascript_uri_in_smil_values_is_rejected() {
        // SMIL <animate> can rewrite attributes at runtime; the old filter
        // only screened static href= attributes.
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <a><animate attributeName="href" values="javascript:alert(1)"/></a>
        </svg>"#;
        assert!(validate_svg(svg).is_err());
    }

    #[test]
    fn malicious_svg_with_smil_href_animation_to_external_url_is_rejected() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <a><animate attributeName="href" values="https://evil.example"/></a>
        </svg>"#;
        assert!(validate_svg(svg).is_err());
    }

    #[test]
    fn malicious_svg_with_data_uri_active_content_is_rejected() {
        let cases: [&[u8]; 2] = [
            b"<svg><text>%3Csvg%3E</text><set attributeName='x' to='data:text/html,<svg/>'/></svg>",
            b"<svg><use xlink:href='data:image/svg+xml;base64,PHN2Zz48L3N2Zz4='/></svg>",
        ];
        for svg in cases {
            assert!(
                validate_svg(svg).is_err(),
                "expected rejection of: {}",
                String::from_utf8_lossy(svg)
            );
        }
    }

    #[test]
    fn malicious_svg_with_style_element_is_rejected() {
        let svg = b"<svg><style>@import url('https://evil.example/x.css')</style></svg>";
        assert!(validate_svg(svg).is_err());
    }

    #[test]
    fn malicious_svg_with_style_attribute_fetch_is_rejected() {
        let svg = br#"<svg><rect style="background:url('https://evil.example/x')"/></svg>"#;
        assert!(validate_svg(svg).is_err());
    }

    #[test]
    fn malicious_svg_with_entity_declaration_is_rejected() {
        // XXE / billion-laughs vector.
        let svg = br#"<!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>
            <svg xmlns="http://www.w3.org/2000/svg"><text>&xxe;</text></svg>"#;
        assert!(validate_svg(svg).is_err());
    }

    #[test]
    fn malicious_svg_with_whitespace_around_href_is_rejected() {
        // Regression: the old substring check `contains("href=")` allowed
        // `href = ` with a space before the equals sign.
        let cases: [&[u8]; 3] = [
            br#"<svg xmlns="http://www.w3.org/2000/svg"><a href = "https://evil.example/x.svg">x</a></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg"><a href
                = "https://evil.example/x.svg">x</a></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg"><use xlink:href = 'https://evil.example/p.svg'/></svg>"#,
        ];
        for svg in cases {
            assert!(
                validate_svg(svg).is_err(),
                "expected rejection of: {}",
                String::from_utf8_lossy(svg)
            );
        }
    }

    #[test]
    fn benign_svg_with_local_fragment_style_reference_is_accepted() {
        // Regression: `style="fill: url(#gradient)"` is the standard way to
        // reference in-document gradients and must not be rejected; only
        // remote fetches (url to a scheme/host) are forbidden.
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg">
            <defs><linearGradient id="grad"><stop stop-color="#00ff00"/></linearGradient></defs>
            <rect style="fill: url(#grad)" width="10" height="10"/>
        </svg>"##;
        assert!(validate_svg(svg).is_ok());
    }

    #[test]
    fn malicious_svg_with_remote_style_url_is_rejected() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <rect style="fill: url(https://evil.example/x)" width="10" height="10"/>
        </svg>"#;
        assert!(validate_svg(svg).is_err());
    }
}
