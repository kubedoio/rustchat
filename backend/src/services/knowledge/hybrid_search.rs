//! Hybrid RAG search: combines Meilisearch full-text + pgvector semantic

use crate::models::knowledge::RetrievedChunk;

/// Result of a hybrid search with fused scores.
pub struct HybridResult {
    pub chunk: RetrievedChunk,
    pub semantic_score: f32,
    pub text_score: f32,
    pub fused_score: f32,
}

/// Reciprocal Rank Fusion (RRF) constant.
pub const RRF_K: f32 = 60.0;

/// Fuse semantic and full-text results using RRF.
pub fn rrf_fuse(
    semantic_results: Vec<RetrievedChunk>,
    text_results: Vec<RetrievedChunk>,
    k: f32,
) -> Vec<HybridResult> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, (RetrievedChunk, f32, f32)> = HashMap::new();

    // Score semantic results by rank
    for (rank, chunk) in semantic_results.iter().enumerate() {
        let key = format!("{}-{}", chunk.document_title, chunk.chunk_text);
        let semantic_score = 1.0 / (rank as f32 + 1.0 + k);
        scores
            .entry(key)
            .and_modify(|(_, sem, _)| *sem += semantic_score)
            .or_insert_with(|| (chunk.clone(), semantic_score, 0.0));
    }

    // Score text results by rank
    for (rank, chunk) in text_results.iter().enumerate() {
        let key = format!("{}-{}", chunk.document_title, chunk.chunk_text);
        let text_score = 1.0 / (rank as f32 + 1.0 + k);
        scores
            .entry(key)
            .and_modify(|(_, _, txt)| *txt += text_score)
            .or_insert_with(|| (chunk.clone(), 0.0, text_score));
    }

    // Build final results sorted by fused score
    let mut hybrid_results: Vec<HybridResult> = scores
        .into_iter()
        .map(|(_, (chunk, semantic_score, text_score))| {
            let fused_score = semantic_score + text_score;
            HybridResult {
                chunk,
                semantic_score,
                text_score,
                fused_score,
            }
        })
        .collect();

    hybrid_results.sort_by(|a, b| b.fused_score.partial_cmp(&a.fused_score).unwrap());
    hybrid_results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(text: &str, title: &str) -> RetrievedChunk {
        RetrievedChunk {
            chunk_text: text.to_string(),
            document_title: title.to_string(),
            document_source_url: None,
            section_title: None,
            similarity: 0.0,
        }
    }

    #[test]
    fn test_rrf_fuse_empty() {
        let result = rrf_fuse(vec![], vec![], RRF_K);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rrf_fuse_semantic_only() {
        let semantic = vec![
            make_chunk("chunk a", "doc 1"),
            make_chunk("chunk b", "doc 2"),
        ];
        let result = rrf_fuse(semantic, vec![], RRF_K);
        assert_eq!(result.len(), 2);
        assert!(result[0].fused_score > result[1].fused_score);
    }

    #[test]
    fn test_rrf_fuse_both_sources() {
        let semantic = vec![
            make_chunk("chunk a", "doc 1"),
            make_chunk("chunk b", "doc 2"),
        ];
        let text = vec![
            make_chunk("chunk b", "doc 2"), // overlap
            make_chunk("chunk c", "doc 3"),
        ];
        let result = rrf_fuse(semantic, text, RRF_K);
        assert_eq!(result.len(), 3);
        // chunk b should be first because it appears in both lists
        assert_eq!(result[0].chunk.chunk_text, "chunk b");
    }
}
