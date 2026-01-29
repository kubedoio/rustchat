use axum::{extract::Path, routing::get, Json, Router};

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::encode_mm_id;

#[derive(serde::Serialize, Clone)]
struct CustomEmoji {
    id: String,
    name: String,
    creator_id: String,
    create_at: i64,
    update_at: i64,
    delete_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
}

/// Standard Unicode emojis that don't need custom images
/// Mattermost mobile expects these to be returned when requesting emoji by name
fn get_standard_emoji(name: &str) -> Option<CustomEmoji> {
    // Check if the name is actually a Unicode emoji character
    // For standard emojis, we return them with a system-generated ID
    if is_unicode_emoji(name) {
        return Some(CustomEmoji {
            id: encode_mm_id_from_string(name),
            name: name.to_string(),
            creator_id: "system".to_string(),
            create_at: 0,
            update_at: 0,
            delete_at: 0,
            image_url: None, // Standard emojis don't need image URLs
        });
    }
    None
}

/// Check if a string is a Unicode emoji
fn is_unicode_emoji(s: &str) -> bool {
    // Check for emoji Unicode ranges
    // This is a simplified check - for production, use a proper emoji detection library
    for c in s.chars() {
        let cp = c as u32;
        // Check common emoji Unicode ranges
        if (0x1F600..=0x1F64F).contains(&cp) ||    // Emoticons
           (0x1F300..=0x1F5FF).contains(&cp) ||    // Misc Symbols and Pictographs
           (0x1F680..=0x1F6FF).contains(&cp) ||    // Transport and Map
           (0x1F1E0..=0x1F1FF).contains(&cp) ||    // Flags
           (0x2600..=0x26FF).contains(&cp) ||      // Misc symbols
           (0x2700..=0x27BF).contains(&cp) ||      // Dingbats
           (0x1F900..=0x1F9FF).contains(&cp) ||    // Supplemental Symbols and Pictographs
           (0x1F018..=0x1F270).contains(&cp) ||    // Chess, playing cards, etc.
           (0x238C..=0x2454).contains(&cp) ||      // Misc
           cp == 0x200D                             // Zero Width Joiner (for combined emojis)
        {
            return true;
        }
    }
    false
}

/// Generate a consistent MM-style ID from a string (for system emojis)
fn encode_mm_id_from_string(s: &str) -> String {
    use sha2::{Sha256, Digest};
    use uuid::Uuid;
    
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    let uuid = Uuid::from_slice(&result[..16]).unwrap_or_else(|_| Uuid::new_v4());
    encode_mm_id(uuid)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/emoji", get(list_emoji))
        .route("/emoji/autocomplete", get(emoji_autocomplete))
        .route("/emoji/name/{name}", get(get_emoji_by_name))
}

async fn list_emoji() -> ApiResult<Json<Vec<CustomEmoji>>> {
    // Return empty list for now - custom emojis not yet implemented
    Ok(Json(vec![]))
}

async fn emoji_autocomplete() -> ApiResult<Json<Vec<CustomEmoji>>> {
    // Return empty list for now - custom emojis not yet implemented
    Ok(Json(vec![]))
}

async fn get_emoji_by_name(Path(name): Path<String>) -> ApiResult<Json<CustomEmoji>> {
    // First check if it's a standard Unicode emoji
    if let Some(emoji) = get_standard_emoji(&name) {
        return Ok(Json(emoji));
    }
    
    // For now, return not found for custom emojis (not yet implemented)
    // In the future, check database for custom emojis here
    Err(AppError::NotFound(format!("Custom emoji '{}' not found", name)))
}
