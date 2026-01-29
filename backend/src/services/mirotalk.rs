use reqwest::Client;
use url::Url;

use crate::error::{ApiResult, AppError};
use crate::models::{CreateMeetingResponse, MeetingsResponse, MiroTalkConfig, MiroTalkMode, MiroTalkStats};

#[derive(Debug, Clone)]
pub struct MiroTalkClient {
    base_url: Url,
    api_key_secret: String,
    mode: MiroTalkMode,
    http: Client,
}

impl MiroTalkClient {
    pub fn new(config: MiroTalkConfig, http: Client) -> ApiResult<Self> {
        let mut base_url = Url::parse(&config.base_url)
            .map_err(|_| AppError::Config("Invalid MiroTalk base URL".to_string()))?;

        // Ensure trailing slash so .join() works correctly with paths
        // Url::join replaces the last path segment if it doesn't end in a slash.
        // We want to treat the user-provided URL as a directory base.
        if !base_url.path().ends_with('/') {
            // This is a bit tricky with the `url` crate.
            // If path is empty, it is "/" which ends with /.
            // If path is "/foo", we want "/foo/".
            // path_segments_mut().pop_if_empty().push("") achieves this.
            if let Ok(mut segments) = base_url.path_segments_mut() {
                segments.pop_if_empty().push("");
            }
        }

        Ok(Self {
            base_url,
            api_key_secret: config.api_key_secret,
            mode: config.mode,
            http,
        })
    }

    pub async fn stats(&self) -> ApiResult<MiroTalkStats> {
        let url = self.base_url.join("api/v1/stats")
            .map_err(|_| AppError::Internal("Failed to build stats URL".to_string()))?;

        let response = self.http.get(url)
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
        // Docs say GET /api/v1/meetings for both SFU and P2P
        let url = self
            .base_url
            .join("api/v1/meetings")
            .map_err(|_| AppError::Internal("Failed to build meetings URL".to_string()))?;

        let response = self
            .http
            .get(url)
            .header("authorization", &self.api_key_secret)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("MiroTalk active meetings connection failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            // Log error but return empty list to not break UI if integration is flaky
            tracing::error!("MiroTalk active meetings error {}: {}", status, text);
            return Ok(vec![]);
        }

        let data = response
            .json::<MeetingsResponse>()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Failed to parse MiroTalk meetings: {}", e))
            })?;

        // Extract meeting identifiers from JSON values
        // Assuming the response is { "meetings": [ "room1", "room2" ] } OR objects
        // If they are strings, we collect them. If objects, we try to find a name/id.
        let rooms: Vec<String> = data.meetings.into_iter().filter_map(|m| {
            if let Some(s) = m.as_str() {
                Some(s.to_string())
            } else if let Some(obj) = m.as_object() {
                // Try common fields
                obj.get("room").or(obj.get("meeting")).and_then(|v| v.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        }).collect();

        Ok(rooms)
    }

    pub async fn create_meeting(&self, room_name: &str) -> ApiResult<String> {
        match self.mode {
            MiroTalkMode::Sfu | MiroTalkMode::P2p => self.create_meeting_api(room_name).await,
            MiroTalkMode::Disabled => {
                Err(AppError::Config("MiroTalk integration is disabled".to_string()))
            }
        }
    }

    async fn create_meeting_api(&self, room_name: &str) -> ApiResult<String> {
        let url = self
            .base_url
            .join("api/v1/meeting")
            .map_err(|_| AppError::Internal("Failed to build meeting URL".to_string()))?;

        // We send room name in body, hoping the API respects it.
        // Even if P2P docs say just POST, passing extra JSON fields shouldn't hurt.
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
                AppError::ExternalService(format!("Failed to create/join meeting: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "MiroTalk create meeting failed {}: {}",
                status, text
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{JoinBehavior, MiroTalkConfig, MiroTalkMode};
    use chrono::Utc;
    use reqwest::Client;

    fn create_config(mode: MiroTalkMode, base_url: &str) -> MiroTalkConfig {
        MiroTalkConfig {
            is_active: true,
            mode,
            base_url: base_url.to_string(),
            api_key_secret: "secret".to_string(),
            default_room_prefix: None,
            join_behavior: JoinBehavior::NewTab,
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn test_base_url_normalization() {
        // Case 1: Root with slash
        let config = create_config(MiroTalkMode::Sfu, "https://example.com/");
        let client = MiroTalkClient::new(config, Client::new()).unwrap();
        assert_eq!(client.base_url.as_str(), "https://example.com/");

        // Case 2: Root without slash
        let config = create_config(MiroTalkMode::Sfu, "https://example.com");
        let client = MiroTalkClient::new(config, Client::new()).unwrap();
        // Url parsing normalizes root to slash automatically
        assert_eq!(client.base_url.as_str(), "https://example.com/");

        // Case 3: Path with slash
        let config = create_config(MiroTalkMode::Sfu, "https://example.com/mirotalk/");
        let client = MiroTalkClient::new(config, Client::new()).unwrap();
        assert_eq!(client.base_url.as_str(), "https://example.com/mirotalk/");

        // Case 4: Path without slash - should be normalized to have slash
        let config = create_config(MiroTalkMode::Sfu, "https://example.com/mirotalk");
        let client = MiroTalkClient::new(config, Client::new()).unwrap();
        assert_eq!(client.base_url.as_str(), "https://example.com/mirotalk/");
    }

    #[test]
    fn test_url_construction_logic() {
        // Verify that .join() works correctly after normalization
        let config = create_config(MiroTalkMode::Sfu, "https://example.com/app");
        let client = MiroTalkClient::new(config, Client::new()).unwrap();

        let url = client.base_url.join("api/v1/meeting").unwrap();
        assert_eq!(url.as_str(), "https://example.com/app/api/v1/meeting");

        let url_meetings = client.base_url.join("api/v1/meetings").unwrap();
        assert_eq!(url_meetings.as_str(), "https://example.com/app/api/v1/meetings");
    }
}
