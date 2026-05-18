//! File upload validation helpers

use crate::constants::*;
use crate::error::AppError;

const ALLOWED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "pdf", "txt", "md", "zip",
];

/// Validate a file upload and return the canonical MIME type and lowercase extension.
pub fn validate_file_upload(filename: &str, data: &[u8]) -> Result<(String, String), AppError> {
    let ext = filename
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();

    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::BadRequest(format!(
            "File extension '.{}' is not allowed",
            ext
        )));
    }

    let max_size = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => MAX_IMAGE_SIZE,
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
        "png" | "jpg" | "jpeg" | "gif" | "webp" => {
            if actual_mime.as_deref() != Some(expected_mime) {
                return Err(AppError::BadRequest(format!(
                    "File content does not match extension '.{}'. Expected {}, got {}",
                    ext,
                    expected_mime,
                    actual_mime.as_deref().unwrap_or("unknown")
                )));
            }
        }
        "pdf" => {
            if actual_mime.as_deref() != Some("application/pdf") {
                return Err(AppError::BadRequest(
                    "File content does not match declared PDF extension".to_string(),
                ));
            }
        }
        "zip" => {
            if actual_mime.as_deref() != Some("application/zip") {
                return Err(AppError::BadRequest(
                    "File content does not match declared ZIP extension".to_string(),
                ));
            }
        }
        "svg" => {
            validate_svg(data)?;
        }
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
        }
        _ => {}
    }

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

    let mime = detect_mime_from_bytes(data).ok_or_else(|| {
        AppError::BadRequest("Could not determine image format".to_string())
    })?;

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
    if data.len() >= 6
        && (data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a"))
    {
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
    let ext = filename
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();

    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::BadRequest(format!(
            "File extension '.{}' is not allowed",
            ext
        )));
    }

    let max_size = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => MAX_IMAGE_SIZE,
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
        "png" | "jpg" | "jpeg" | "gif" | "webp" => {
            if actual_mime.as_deref() != Some(expected_mime) {
                return Err(AppError::BadRequest(format!(
                    "File content does not match extension '.{}'. Expected {}, got {}",
                    ext,
                    expected_mime,
                    actual_mime.as_deref().unwrap_or("unknown")
                )));
            }
        }
        "pdf" => {
            if actual_mime.as_deref() != Some("application/pdf") {
                return Err(AppError::BadRequest(
                    "File content does not match declared PDF extension".to_string(),
                ));
            }
        }
        "zip" => {
            if actual_mime.as_deref() != Some("application/zip") {
                return Err(AppError::BadRequest(
                    "File content does not match declared ZIP extension".to_string(),
                ));
            }
        }
        _ => {}
    }

    Ok((expected_mime.to_string(), ext))
}

fn validate_svg(data: &[u8]) -> Result<(), AppError> {
    let text = std::str::from_utf8(data).map_err(|_| {
        AppError::BadRequest("SVG must be valid UTF-8".to_string())
    })?;

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
    if lower.contains("<script") || lower.contains("</script>") {
        return Err(AppError::BadRequest(
            "SVG contains forbidden script elements".to_string(),
        ));
    }

    Ok(())
}
