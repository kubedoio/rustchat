//! Post service: creation, system messages, querying and rendering.
//!
//! This module is a thin facade over cohesive submodules; the public items are
//! re-exported so callers (and the module path `crate::services::posts::*`)
//! are unchanged by the decomposition.

mod posting;
mod query;
mod system;

use regex::Regex;

pub use posting::create_post;
pub use query::{
    get_post_by_id, get_posts, get_thread, normalize_post_avatar_urls, populate_files, PostsQuery,
    ThreadQuery,
};
pub use system::create_system_message;

/// Parse @mentions from a message, excluding code blocks and URLs.
pub(crate) fn parse_mentions(message: &str) -> Vec<String> {
    let mention_re = Regex::new(r"@([a-zA-Z0-9_\-\.]+)").expect("valid regex");
    let mut mentions = Vec::new();
    let mut in_code_block = false;

    for line in message.lines() {
        // Track fenced code blocks (```)
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Find mentions in this line, skipping inline code segments
        for mat in mention_re.find_iter(line) {
            let start = mat.start();
            let prefix = &line[..start];
            // Skip inline code: odd number of backticks before mention
            if prefix.matches('`').count() % 2 == 1 {
                continue;
            }
            // Skip URLs (http://... or https://...)
            if prefix.ends_with("http://") || prefix.ends_with("https://") {
                continue;
            }
            mentions.push(mat.as_str()[1..].to_string());
        }
    }

    mentions
}
