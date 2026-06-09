//! Document chunking strategies
//!
//! Splits extracted text into chunks suitable for embedding.

/// Configuration for chunking.
pub struct ChunkConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
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

            let start_byte = text
                .char_indices()
                .nth(start)
                .map(|(i, _)| i)
                .unwrap_or(0);
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
