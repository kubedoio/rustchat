//! Document chunking strategies
//!
//! Splits extracted text into chunks suitable for embedding.

/// Configuration for chunking.
pub struct ChunkConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 50,
        }
    }
}

/// A single text chunk.
pub struct Chunk {
    pub text: String,
    pub token_count: usize,
    pub section_title: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Trait for text chunkers.
pub trait Chunker: Send + Sync {
    /// Chunk the given text according to the config.
    fn chunk(&self, text: &str, config: &ChunkConfig) -> Vec<Chunk>;
}

/// Select the best chunker for a given MIME type.
pub fn select_chunker(_mime_type: &str) -> Box<dyn Chunker> {
    // For now, use sliding window for all types.
    // Future: select MarkdownChunker for markdown, CodeChunker for code, etc.
    Box::new(SlidingWindowChunker)
}

/// Simple sliding-window chunker that splits on character boundaries.
pub struct SlidingWindowChunker;

impl Chunker for SlidingWindowChunker {
    fn chunk(&self, text: &str, config: &ChunkConfig) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let char_count = text.chars().count();
        if char_count == 0 {
            return chunks;
        }

        let step = if config.chunk_size > config.chunk_overlap {
            config.chunk_size - config.chunk_overlap
        } else {
            1
        };

        let mut start = 0;
        while start < char_count {
            let end = (start + config.chunk_size).min(char_count);

            let start_byte = text.char_indices().nth(start).map(|(i, _)| i).unwrap_or(0);
            let end_byte = text
                .char_indices()
                .nth(end.saturating_sub(1))
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(text.len());

            let chunk_text: String = text[start_byte..end_byte].to_string();

            chunks.push(Chunk {
                text: chunk_text,
                token_count: end - start, // char count as token proxy
                section_title: None,
                start_byte,
                end_byte,
            });

            if end == char_count {
                break;
            }
            start += step;
        }

        chunks
    }
}

/// Markdown-aware chunker that splits on headers.
pub struct MarkdownChunker;

impl Chunker for MarkdownChunker {
    fn chunk(&self, text: &str, config: &ChunkConfig) -> Vec<Chunk> {
        let headers = find_markdown_headers(text);
        if headers.is_empty() {
            // No headers found: fall back to sliding window.
            return SlidingWindowChunker.chunk(text, config);
        }

        let mut chunks = Vec::new();
        for i in 0..headers.len() {
            let (header_start, title) = &headers[i];
            let section_start = *header_start;
            let section_end = if i + 1 < headers.len() {
                headers[i + 1].0
            } else {
                text.len()
            };

            let section_text = &text[section_start..section_end];
            let section_char_count = section_text.chars().count();

            if section_char_count <= config.chunk_size {
                let start_byte = section_start;
                let end_byte = section_end;
                chunks.push(Chunk {
                    text: section_text.trim_end().to_string(),
                    token_count: section_char_count,
                    section_title: Some(title.clone()),
                    start_byte,
                    end_byte,
                });
            } else {
                // Section too large: slide a window over it, preserving title.
                let step = if config.chunk_size > config.chunk_overlap {
                    config.chunk_size - config.chunk_overlap
                } else {
                    1
                };

                let mut start = 0;
                while start < section_char_count {
                    let end = (start + config.chunk_size).min(section_char_count);

                    let local_start_byte = section_text
                        .char_indices()
                        .nth(start)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let local_end_byte = section_text
                        .char_indices()
                        .nth(end.saturating_sub(1))
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(section_text.len());

                    let chunk_text = section_text[local_start_byte..local_end_byte].to_string();

                    chunks.push(Chunk {
                        text: chunk_text,
                        token_count: end - start,
                        section_title: Some(title.clone()),
                        start_byte: section_start + local_start_byte,
                        end_byte: section_start + local_end_byte,
                    });

                    if end == section_char_count {
                        break;
                    }
                    start += step;
                }
            }
        }

        chunks
    }
}

/// Find all markdown headers in the text.
/// Returns a vector of (byte_offset, title) pairs.
fn find_markdown_headers(text: &str) -> Vec<(usize, String)> {
    let mut headers = Vec::new();
    for line in text.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            let offset = text.find(line).unwrap_or(0);
            headers.push((offset, title.trim().to_string()));
        } else if let Some(title) = line.strip_prefix("## ") {
            let offset = text.find(line).unwrap_or(0);
            headers.push((offset, title.trim().to_string()));
        } else if let Some(title) = line.strip_prefix("### ") {
            let offset = text.find(line).unwrap_or(0);
            headers.push((offset, title.trim().to_string()));
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_empty_text() {
        let chunker = SlidingWindowChunker;
        let config = ChunkConfig {
            chunk_size: 10,
            chunk_overlap: 2,
        };
        let chunks = chunker.chunk("", &config);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_sliding_window_short_text() {
        let chunker = SlidingWindowChunker;
        let config = ChunkConfig {
            chunk_size: 100,
            chunk_overlap: 10,
        };
        let chunks = chunker.chunk("Hello world", &config);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world");
    }

    #[test]
    fn test_sliding_window_multiple_chunks() {
        let chunker = SlidingWindowChunker;
        let config = ChunkConfig {
            chunk_size: 5,
            chunk_overlap: 1,
        };
        let text = "one two three four five six seven eight nine ten";
        let chunks = chunker.chunk(text, &config);
        assert!(chunks.len() > 1);
        // Verify overlap: first chunk's end should overlap with second chunk's start
        for window in chunks.windows(2) {
            let first = &window[0];
            let second = &window[1];
            // The second chunk should start before the first chunk ends
            assert!(
                second.start_byte < first.end_byte || second.start_byte == first.start_byte,
                "Chunks should overlap or be contiguous"
            );
        }
    }

    #[test]
    fn test_markdown_chunker_with_headers() {
        let chunker = MarkdownChunker;
        let config = ChunkConfig {
            chunk_size: 100,
            chunk_overlap: 10,
        };
        let text = "# Introduction\nThis is the intro.\n\n## Details\nThese are the details.\n\n## Conclusion\nThe end.";
        let chunks = chunker.chunk(text, &config);

        // Should have at least 3 chunks (one per section)
        assert!(
            chunks.len() >= 3,
            "Markdown chunker should split on headers"
        );

        // Verify section titles are captured
        let titles: Vec<_> = chunks
            .iter()
            .filter_map(|c| c.section_title.as_ref())
            .collect();
        assert!(titles.contains(&&"Introduction".to_string()));
        assert!(titles.contains(&&"Details".to_string()));
        assert!(titles.contains(&&"Conclusion".to_string()));
    }

    #[test]
    fn test_markdown_chunker_fallback() {
        let chunker = MarkdownChunker;
        let config = ChunkConfig {
            chunk_size: 5,
            chunk_overlap: 1,
        };
        let text = "This is a long paragraph without any headers at all it just keeps going";
        let chunks = chunker.chunk(text, &config);
        assert!(
            chunks.len() > 1,
            "Should fall back to sliding window when no headers"
        );
    }

    #[test]
    fn test_select_chunker() {
        assert!(
            select_chunker("text/markdown")
                .chunk("# Test", &ChunkConfig::default())
                .len()
                > 0
        );
        assert!(
            select_chunker("text/plain")
                .chunk("Hello", &ChunkConfig::default())
                .len()
                > 0
        );
    }
}
