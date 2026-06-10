//! Document text extraction
//!
//! Extracts plain text from various document formats for indexing.

use std::io::{Cursor, Read};

/// Extractor error type
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("Unsupported MIME type: {0}")]
    UnsupportedMimeType(String),
    #[error("PDF extraction failed: {0}")]
    PdfExtractError(String),
    #[error("HTML extraction failed: {0}")]
    HtmlExtractError(String),
    #[error("DOCX extraction failed: {0}")]
    DocxExtractError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Trait for document text extractors.
pub trait DocumentExtractor: Send + Sync {
    /// Extract plain text from document bytes.
    fn extract(&self, data: &[u8]) -> Result<String, ExtractError>;
    /// Returns the MIME types this extractor supports.
    fn supported_types(&self) -> &'static [&'static str];
}

/// Registry of extractors. Selects the appropriate extractor by MIME type.
pub struct ExtractorRegistry {
    extractors: Vec<Box<dyn DocumentExtractor>>,
}

impl ExtractorRegistry {
    pub fn default_registry() -> Self {
        Self {
            extractors: vec![
                Box::new(PlainTextExtractor),
                Box::new(MarkdownExtractor),
                Box::new(PdfExtractor),
                Box::new(HtmlExtractor),
                Box::new(DocxExtractor),
            ],
        }
    }

    pub fn extract(&self, data: &[u8], mime_type: &str) -> Result<String, ExtractError> {
        for extractor in &self.extractors {
            if extractor.supported_types().contains(&mime_type) {
                return extractor.extract(data);
            }
        }
        // Fallback: try plain text for text/* types
        if mime_type.starts_with("text/") {
            return PlainTextExtractor.extract(data);
        }
        Err(ExtractError::UnsupportedMimeType(mime_type.to_string()))
    }
}

pub struct PlainTextExtractor;
impl DocumentExtractor for PlainTextExtractor {
    #[tracing::instrument(skip(self, data), fields(mime_type = "text/plain"))]
    fn extract(&self, data: &[u8]) -> Result<String, ExtractError> {
        Ok(String::from_utf8_lossy(data).into_owned())
    }
    fn supported_types(&self) -> &'static [&'static str] {
        &["text/plain", "text/x-rust", "text/x-python"]
    }
}

pub struct MarkdownExtractor;
impl DocumentExtractor for MarkdownExtractor {
    #[tracing::instrument(skip(self, data), fields(mime_type = "text/markdown"))]
    fn extract(&self, data: &[u8]) -> Result<String, ExtractError> {
        Ok(String::from_utf8_lossy(data).into_owned())
    }
    fn supported_types(&self) -> &'static [&'static str] {
        &["text/markdown", "text/x-markdown"]
    }
}

pub struct PdfExtractor;
impl DocumentExtractor for PdfExtractor {
    #[tracing::instrument(skip(self, data), fields(mime_type = "application/pdf"))]
    fn extract(&self, data: &[u8]) -> Result<String, ExtractError> {
        pdf_extract::extract_text_from_mem(data)
            .map_err(|e| ExtractError::PdfExtractError(e.to_string()))
    }
    fn supported_types(&self) -> &'static [&'static str] {
        &["application/pdf"]
    }
}

pub struct HtmlExtractor;
impl DocumentExtractor for HtmlExtractor {
    #[tracing::instrument(skip(self, data), fields(mime_type = "text/html"))]
    fn extract(&self, data: &[u8]) -> Result<String, ExtractError> {
        let text = html2text::from_read(Cursor::new(data), 80)
            .map_err(|e| ExtractError::HtmlExtractError(e.to_string()))?;
        Ok(text)
    }
    fn supported_types(&self) -> &'static [&'static str] {
        &["text/html"]
    }
}

pub struct DocxExtractor;
impl DocumentExtractor for DocxExtractor {
    #[tracing::instrument(
        skip(self, data),
        fields(
            mime_type = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
    )]
    fn extract(&self, data: &[u8]) -> Result<String, ExtractError> {
        let cursor = Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| ExtractError::DocxExtractError(e.to_string()))?;

        let mut doc_xml = archive
            .by_name("word/document.xml")
            .map_err(|e| ExtractError::DocxExtractError(e.to_string()))?;
        let mut xml_content = String::new();
        doc_xml
            .read_to_string(&mut xml_content)
            .map_err(|e| ExtractError::DocxExtractError(e.to_string()))?;

        // Parse XML and extract text from <w:t> tags
        let mut texts = Vec::new();
        let mut reader = quick_xml::Reader::from_str(&xml_content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut in_text_element = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(e)) => {
                    if e.name().as_ref() == b"w:t" {
                        in_text_element = true;
                    }
                }
                Ok(quick_xml::events::Event::Text(e)) => {
                    if in_text_element {
                        if let Ok(text) = e.unescape() {
                            texts.push(text.into_owned());
                        }
                    }
                }
                Ok(quick_xml::events::Event::End(e)) => {
                    if e.name().as_ref() == b"w:t" {
                        in_text_element = false;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => return Err(ExtractError::DocxExtractError(e.to_string())),
            }
            buf.clear();
        }

        Ok(texts.join(""))
    }
    fn supported_types(&self) -> &'static [&'static str] {
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]
    }
}
