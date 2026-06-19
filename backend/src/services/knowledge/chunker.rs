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
    fn test_select_chunker() {
        assert!(!select_chunker("text/markdown")
            .chunk("# Test", &ChunkConfig::default())
            .is_empty());
        assert!(!select_chunker("text/plain")
            .chunk("Hello", &ChunkConfig::default())
            .is_empty());
    }
}
