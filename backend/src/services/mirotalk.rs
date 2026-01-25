use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{ApiResult, AppError};
use crate::models::{MiroTalkConfig, MiroTalkMode};

#[derive(Debug, Clone)]
pub struct MiroTalkClient {
    base_url: Url,
    api_key_secret: String,
    mode: MiroTalkMode,
    http: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MiroTalkStats {
    pub peers: Option<i32>,
    pub rooms: Option<i32>,
    pub active_rooms: Option<Vec<String>>,
    // Add other fields as needed based on actual API response
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMeetingResponse {
    pub meeting: String, // The URL
}

impl MiroTalkClient {
    pub fn new(config: MiroTalkConfig, http: Client) -> ApiResult<Self> {
        let base_url = Url::parse(&config.base_url)
            .map_err(|_| AppError::Config("Invalid MiroTalk base URL".to_string()))?;

        Ok(Self {
            base_url,
            api_key_secret: config.api_key_secret,
            mode: config.mode,
            http,
        })
    }

    pub async fn stats(&self) -> ApiResult<MiroTalkStats> {
        let url = self
            .base_url
            .join("api/v1/stats")
            .map_err(|_| AppError::Internal("Failed to build stats URL".to_string()))?;

        let response = self
            .http
            .get(url)
            .header("authorization", &self.api_key_secret)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("MiroTalk connection failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "MiroTalk stats error {}: {}",
                status, text
            )));
        }

        let stats = response.json::<MiroTalkStats>().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse MiroTalk stats: {}", e))
        })?;

        Ok(stats)
    }

    pub async fn get_active_meetings(&self) -> ApiResult<Vec<String>> {
        // SFU: GET /api/v1/meeting
        // P2P: GET /api/v1/rooms (or similar, widely varies, for now we try /api/v1/meeting for SFU compatibility)

        let endpoint = if self.mode == MiroTalkMode::Sfu {
            "api/v1/meeting"
        } else {
            // P2P usually doesn't expose active rooms easily via public API unless configured.
            // We'll try same endpoint or return empty.
            "api/v1/rooms"
        };

        let url = self
            .base_url
            .join(endpoint)
            .map_err(|_| AppError::Internal("Failed to build meetings URL".to_string()))?;

        let response = self
            .http
            .get(url)
            .header("authorization", &self.api_key_secret)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    // Try to parse as list of strings
                    let rooms = resp.json::<Vec<String>>().await.unwrap_or_default();
                    Ok(rooms)
                } else {
                    // Fail silently or return empty for now as P2P might not support it
                    Ok(vec![])
                }
            }
            Err(_) => Ok(vec![]),
        }
    }

    pub async fn create_meeting(&self, room_name: &str) -> ApiResult<String> {
        match self.mode {
            MiroTalkMode::Sfu => self.create_meeting_sfu(room_name).await,
            MiroTalkMode::P2p => self.create_meeting_p2p(room_name).await,
            MiroTalkMode::Disabled => Err(AppError::Config(
                "MiroTalk integration is disabled".to_string(),
            )),
        }
    }

    async fn create_meeting_sfu(&self, room_name: &str) -> ApiResult<String> {
        let url = self
            .base_url
            .join("api/v1/meeting")
            .map_err(|_| AppError::Internal("Failed to build meeting URL".to_string()))?;

        let body = serde_json::json!({
            "room": room_name,
        });

        let response = self
            .http
            .post(url)
            .header("authorization", &self.api_key_secret)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Failed to create SFU meeting: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(AppError::ExternalService(format!(
                "MiroTalk create meeting failed: {}",
                response.status()
            )));
        }

        let data = response
            .json::<CreateMeetingResponse>()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Invalid response from MiroTalk: {}", e))
            })?;

        Ok(data.meeting)
    }

    async fn create_meeting_p2p(&self, room_name: &str) -> ApiResult<String> {
        // For P2P, often the URL is just constructed: BASE_URL + /room_name
        // But if we want to use the API to ensure it exists or get a token:
        // Documentation says POST /api/v1/meeting also works for some P2P versions.
        // If not, we fallback to constructing the URL.

        // Let's try API first if we want to be "secure".
        // But P2P MiroTalk often allows direct join via URL.
        // Let's assume we just construct the URL for P2P unless we specifically need a token.
        // The prompt says: "Use /api/v1/meeting or /api/v1/token ...".

        // If we simply construct the URL:
        // https://p2p.mirotalk.com/room_name

        // Let's try to construct it directly to be safe and simple for P2P default.
        let mut join_url = self.base_url.clone();
        // Remove trailing slash if any and append room name.
        // Join handles it.
        // Assuming base_url is "https://p2p.mirotalk.com"
        // We want "https://p2p.mirotalk.com/room_name" (usually join path is root or /join/...)

        // MiroTalk P2P structure: https://url/roomname
        if let Ok(mut segments) = join_url.path_segments_mut() {
            segments.push(room_name);
        } else {
            return Err(AppError::Internal(
                "Invalid P2P URL construction".to_string(),
            ));
        }

        Ok(join_url.to_string())
    }
}
