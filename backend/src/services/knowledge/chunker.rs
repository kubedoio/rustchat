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
        if text.is_empty() {
            return chunks;
        }

        // Precompute byte offset + UTF-8 length per character once so chunk
        // slicing is O(1) per chunk. Scanning `char_indices().nth(n)` from
        // the start per chunk made this O(n^2) and pinned a worker for
        // minutes on large documents.
        let char_data: Vec<(usize, usize)> = text
            .char_indices()
            .map(|(i, c)| (i, c.len_utf8()))
            .collect();
        let char_count = char_data.len();

        let step = if config.chunk_size > config.chunk_overlap {
            config.chunk_size - config.chunk_overlap
        } else {
            1
        };

        let mut start = 0;
        while start < char_count {
            let end = (start + config.chunk_size).min(char_count);

            let (start_byte, _) = char_data[start];
            let (end_char_byte, end_char_len) = char_data[end - 1];
            let end_byte = end_char_byte + end_char_len;

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

    #[test]
    fn test_sliding_window_multibyte_boundaries() {
        // Chunks must split on character boundaries for multi-byte text.
        let chunker = SlidingWindowChunker;
        let config = ChunkConfig {
            chunk_size: 3,
            chunk_overlap: 1,
        };
        let text = "👍中文消息内容测试"; // 4-byte emoji + 8 CJK chars
        let chunks = chunker.chunk(text, &config);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            // If slicing ever landed inside a character, from_utf8 would
            // have already panicked on to_string(); additionally assert the
            // text round-trips.
            assert_eq!(chunk.text.chars().count(), chunk.token_count);
        }
        // First chunk starts at the emoji.
        assert!(chunks[0].text.starts_with('👍'));
    }

    #[test]
    fn test_sliding_window_large_input_is_linear() {
        // 200k chars with a small step would take minutes with the old
        // per-chunk `char_indices().nth(n)` rescan. Completing quickly here
        // is the regression guard.
        let chunker = SlidingWindowChunker;
        let config = ChunkConfig {
            chunk_size: 64,
            chunk_overlap: 8,
        };
        let text: String = "abcdefghij".repeat(20_000);
        let chunks = chunker.chunk(&text, &config);
        assert!(chunks.len() > 1_000);
    }
}
