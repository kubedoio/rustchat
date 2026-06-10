//! Error types for rustchat
//!
//! Provides structured error handling with HTTP status code mapping.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Application error types
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    // ── Typed variants for common NotFound errors ──
    #[error("Not found: Post not found")]
    PostNotFound,

    #[error("Not found: Channel not found")]
    ChannelNotFound,

    #[error("Not found: User not found")]
    UserNotFound,

    #[error("Not found: Team not found")]
    TeamNotFound,

    #[error("Not found: Thread not found")]
    ThreadNotFound,

    #[error("Not found: Member not found")]
    MemberNotFound,

    #[error("Not found: File not found")]
    FileNotFound,

    #[error("Not found: Scheduled post not found")]
    ScheduledPostNotFound,

    #[error("Not found: Webhook not found")]
    WebhookNotFound,

    #[error("Not found: Group not found")]
    GroupNotFound,

    #[error("Not found: Emoji not found")]
    EmojiNotFound,

    #[error("Not found: Bot not found")]
    BotNotFound,

    #[error("Not found: Bookmark not found")]
    BookmarkNotFound,

    #[error("Not found: Command not found")]
    CommandNotFound,

    #[error("Not found: Category not found")]
    CategoryNotFound,

    #[error("Not found: Upload session not found")]
    UploadNotFound,

    #[error("Not found: No active call in this channel")]
    NoActiveCall,

    #[error("Not found: Provider not found")]
    ProviderNotFound,

    #[error("Not found: Terms not found")]
    TermsNotFound,

    #[error("Not found: Template version not found")]
    TemplateVersionNotFound,

    #[error("Not found: Run not found")]
    RunNotFound,

    #[error("Not found: Participant not found in call")]
    ParticipantNotFound,

    #[error("Not found: Before post not found")]
    BeforePostNotFound,

    #[error("Not found: After post not found")]
    AfterPostNotFound,

    // ── Typed variants for common BadRequest errors ──
    #[error("Bad request: Invalid post_id")]
    InvalidPostId,

    #[error("Bad request: Invalid channel_id")]
    InvalidChannelId,

    #[error("Bad request: Channel is already archived")]
    ChannelAlreadyArchived,

    #[error("Bad request: Channel is not archived")]
    ChannelNotArchived,

    #[error("Bad request: Invalid user_id")]
    InvalidUserId,

    #[error("Bad request: Invalid team_id")]
    InvalidTeamId,

    #[error("Bad request: Invalid id")]
    InvalidId,

    #[error("Bad request: Invalid cursor")]
    InvalidCursor,

    #[error("Bad request: Invalid root_id")]
    InvalidRootId,

    #[error("Bad request: Invalid group_id")]
    InvalidGroupId,

    #[error("Bad request: Invalid file_id")]
    InvalidFileId,

    #[error("Bad request: Invalid thread_id")]
    InvalidThreadId,

    #[error("Bad request: Invalid session_id")]
    InvalidSessionId,

    #[error("Bad request: Invalid bookmark_id")]
    InvalidBookmarkId,

    #[error("Bad request: Invalid emoji_id")]
    InvalidEmojiId,

    #[error("Bad request: Invalid bot_id")]
    InvalidBotId,

    #[error("Bad request: Invalid webhook_id")]
    InvalidWebhookId,

    #[error("Bad request: Invalid command_id")]
    InvalidCommandId,

    #[error("Bad request: Invalid scheme_id")]
    InvalidSchemeId,

    #[error("Bad request: Invalid role_id")]
    InvalidRoleId,

    #[error("Bad request: Invalid upload_id")]
    InvalidUploadId,

    #[error("Bad request: Invalid plugin_id")]
    InvalidPluginId,

    // ── Typed variants for common Validation errors ──
    #[error("Validation error: Invalid post_id")]
    ValidationInvalidPostId,

    #[error("Validation error: Invalid channel_id")]
    ValidationInvalidChannelId,

    #[error("Validation error: Invalid user_id")]
    ValidationInvalidUserId,

    #[error("Validation error: Invalid team_id")]
    ValidationInvalidTeamId,

    #[error("Validation error: Invalid root_id")]
    ValidationInvalidRootId,

    #[error("Validation error: Invalid scheduled_at")]
    ValidationInvalidScheduledAt,

    #[error("Validation error: Invalid scheduled_post_id")]
    ValidationInvalidScheduledPostId,

    #[error("Validation error: Invalid target_at")]
    ValidationInvalidTargetAt,

    #[error("Validation error: Invalid group_id")]
    ValidationInvalidGroupId,

    #[error("Validation error: Invalid file_id")]
    ValidationInvalidFileId,

    #[error("Validation error: Invalid thread_id")]
    ValidationInvalidThreadId,

    #[error("Validation error: Invalid session_id")]
    ValidationInvalidSessionId,

    #[error("Validation error: Invalid bookmark_id")]
    ValidationInvalidBookmarkId,

    #[error("Validation error: Invalid emoji_id")]
    ValidationInvalidEmojiId,

    #[error("Validation error: Invalid bot_id")]
    ValidationInvalidBotId,

    #[error("Validation error: Invalid webhook_id")]
    ValidationInvalidWebhookId,

    #[error("Validation error: Invalid command_id")]
    ValidationInvalidCommandId,

    #[error("Validation error: Invalid scheme_id")]
    ValidationInvalidSchemeId,

    #[error("Validation error: Invalid role_id")]
    ValidationInvalidRoleId,

    #[error("Validation error: Invalid upload_id")]
    ValidationInvalidUploadId,

    #[error("Validation error: Invalid plugin_id")]
    ValidationInvalidPluginId,

    #[error("Validation error: Invalid hook_id")]
    ValidationInvalidHookId,

    // ── Typed variants for common Forbidden errors ──
    #[error("Forbidden: Not a member of this channel")]
    NotAMember,

    #[error("Forbidden: Not a member of this team")]
    NotOnTeam,

    #[error("Forbidden: Admin access required")]
    AdminRequired,

    #[error("Forbidden: Insufficient permissions")]
    InsufficientPermissions,

    #[error("Forbidden: Cannot access another user's posts")]
    CannotAccessOthersPosts,

    #[error("Forbidden: Cannot edit others' posts")]
    CannotEditOthersPosts,

    #[error("Forbidden: Cannot delete this post")]
    CannotDeletePost,

    #[error("Forbidden: Cannot delete others' posts")]
    CannotDeleteOthersPosts,

    #[error("Forbidden: Cannot update another user's scheduled post")]
    CannotUpdateOthersScheduledPost,

    #[error("Forbidden: Cannot delete another user's scheduled post")]
    CannotDeleteOthersScheduledPost,

    #[error("Forbidden: Cannot acknowledge for another user")]
    CannotAcknowledgeForOther,

    #[error("Forbidden: You are not in this call")]
    NotInCall,

    #[error("Forbidden: Cannot access this bot")]
    CannotAccessBot,
}

/// Error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl AppError {
    /// Get the error code string
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::Conflict(_) => "CONFLICT",
            AppError::Internal(_) => "INTERNAL_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Redis(_) => "REDIS_ERROR",
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::Config(_) => "CONFIG_ERROR",
            AppError::ExternalService(_) => "EXTERNAL_SERVICE_ERROR",
            AppError::TooManyRequests(_) => "TOO_MANY_REQUESTS",
            AppError::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            // NotFound typed variants
            AppError::PostNotFound => "NOT_FOUND",
            AppError::ChannelNotFound => "NOT_FOUND",
            AppError::UserNotFound => "NOT_FOUND",
            AppError::TeamNotFound => "NOT_FOUND",
            AppError::ThreadNotFound => "NOT_FOUND",
            AppError::MemberNotFound => "NOT_FOUND",
            AppError::FileNotFound => "NOT_FOUND",
            AppError::ScheduledPostNotFound => "NOT_FOUND",
            AppError::WebhookNotFound => "NOT_FOUND",
            AppError::GroupNotFound => "NOT_FOUND",
            AppError::EmojiNotFound => "NOT_FOUND",
            AppError::BotNotFound => "NOT_FOUND",
            AppError::BookmarkNotFound => "NOT_FOUND",
            AppError::CommandNotFound => "NOT_FOUND",
            AppError::CategoryNotFound => "NOT_FOUND",
            AppError::UploadNotFound => "NOT_FOUND",
            AppError::NoActiveCall => "NOT_FOUND",
            AppError::ProviderNotFound => "NOT_FOUND",
            AppError::TermsNotFound => "NOT_FOUND",
            AppError::TemplateVersionNotFound => "NOT_FOUND",
            AppError::RunNotFound => "NOT_FOUND",
            AppError::ParticipantNotFound => "NOT_FOUND",
            AppError::BeforePostNotFound => "NOT_FOUND",
            AppError::AfterPostNotFound => "NOT_FOUND",
            // BadRequest typed variants
            AppError::InvalidPostId => "BAD_REQUEST",
            AppError::InvalidChannelId => "BAD_REQUEST",
            AppError::ChannelAlreadyArchived => "BAD_REQUEST",
            AppError::ChannelNotArchived => "BAD_REQUEST",
            AppError::InvalidUserId => "BAD_REQUEST",
            AppError::InvalidTeamId => "BAD_REQUEST",
            AppError::InvalidId => "BAD_REQUEST",
            AppError::InvalidCursor => "BAD_REQUEST",
            AppError::InvalidRootId => "BAD_REQUEST",
            AppError::InvalidGroupId => "BAD_REQUEST",
            AppError::InvalidFileId => "BAD_REQUEST",
            AppError::InvalidThreadId => "BAD_REQUEST",
            AppError::InvalidSessionId => "BAD_REQUEST",
            AppError::InvalidBookmarkId => "BAD_REQUEST",
            AppError::InvalidEmojiId => "BAD_REQUEST",
            AppError::InvalidBotId => "BAD_REQUEST",
            AppError::InvalidWebhookId => "BAD_REQUEST",
            AppError::InvalidCommandId => "BAD_REQUEST",
            AppError::InvalidSchemeId => "BAD_REQUEST",
            AppError::InvalidRoleId => "BAD_REQUEST",
            AppError::InvalidUploadId => "BAD_REQUEST",
            AppError::InvalidPluginId => "BAD_REQUEST",
            // Validation typed variants
            AppError::ValidationInvalidPostId => "VALIDATION_ERROR",
            AppError::ValidationInvalidChannelId => "VALIDATION_ERROR",
            AppError::ValidationInvalidUserId => "VALIDATION_ERROR",
            AppError::ValidationInvalidTeamId => "VALIDATION_ERROR",
            AppError::ValidationInvalidRootId => "VALIDATION_ERROR",
            AppError::ValidationInvalidScheduledAt => "VALIDATION_ERROR",
            AppError::ValidationInvalidScheduledPostId => "VALIDATION_ERROR",
            AppError::ValidationInvalidTargetAt => "VALIDATION_ERROR",
            AppError::ValidationInvalidGroupId => "VALIDATION_ERROR",
            AppError::ValidationInvalidFileId => "VALIDATION_ERROR",
            AppError::ValidationInvalidThreadId => "VALIDATION_ERROR",
            AppError::ValidationInvalidSessionId => "VALIDATION_ERROR",
            AppError::ValidationInvalidBookmarkId => "VALIDATION_ERROR",
            AppError::ValidationInvalidEmojiId => "VALIDATION_ERROR",
            AppError::ValidationInvalidBotId => "VALIDATION_ERROR",
            AppError::ValidationInvalidWebhookId => "VALIDATION_ERROR",
            AppError::ValidationInvalidCommandId => "VALIDATION_ERROR",
            AppError::ValidationInvalidSchemeId => "VALIDATION_ERROR",
            AppError::ValidationInvalidRoleId => "VALIDATION_ERROR",
            AppError::ValidationInvalidUploadId => "VALIDATION_ERROR",
            AppError::ValidationInvalidPluginId => "VALIDATION_ERROR",
            AppError::ValidationInvalidHookId => "VALIDATION_ERROR",
            // Forbidden typed variants
            AppError::NotAMember => "FORBIDDEN",
            AppError::NotOnTeam => "FORBIDDEN",
            AppError::AdminRequired => "FORBIDDEN",
            AppError::InsufficientPermissions => "FORBIDDEN",
            AppError::CannotAccessOthersPosts => "FORBIDDEN",
            AppError::CannotEditOthersPosts => "FORBIDDEN",
            AppError::CannotDeletePost => "FORBIDDEN",
            AppError::CannotDeleteOthersPosts => "FORBIDDEN",
            AppError::CannotUpdateOthersScheduledPost => "FORBIDDEN",
            AppError::CannotDeleteOthersScheduledPost => "FORBIDDEN",
            AppError::CannotAcknowledgeForOther => "FORBIDDEN",
            AppError::NotInCall => "FORBIDDEN",
            AppError::CannotAccessBot => "FORBIDDEN",
        }
    }

    /// Get the HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ExternalService(_) => StatusCode::BAD_GATEWAY,
            AppError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            // NotFound typed variants
            AppError::PostNotFound => StatusCode::NOT_FOUND,
            AppError::ChannelNotFound => StatusCode::NOT_FOUND,
            AppError::UserNotFound => StatusCode::NOT_FOUND,
            AppError::TeamNotFound => StatusCode::NOT_FOUND,
            AppError::ThreadNotFound => StatusCode::NOT_FOUND,
            AppError::MemberNotFound => StatusCode::NOT_FOUND,
            AppError::FileNotFound => StatusCode::NOT_FOUND,
            AppError::ScheduledPostNotFound => StatusCode::NOT_FOUND,
            AppError::WebhookNotFound => StatusCode::NOT_FOUND,
            AppError::GroupNotFound => StatusCode::NOT_FOUND,
            AppError::EmojiNotFound => StatusCode::NOT_FOUND,
            AppError::BotNotFound => StatusCode::NOT_FOUND,
            AppError::BookmarkNotFound => StatusCode::NOT_FOUND,
            AppError::CommandNotFound => StatusCode::NOT_FOUND,
            AppError::CategoryNotFound => StatusCode::NOT_FOUND,
            AppError::UploadNotFound => StatusCode::NOT_FOUND,
            AppError::NoActiveCall => StatusCode::NOT_FOUND,
            AppError::ProviderNotFound => StatusCode::NOT_FOUND,
            AppError::TermsNotFound => StatusCode::NOT_FOUND,
            AppError::TemplateVersionNotFound => StatusCode::NOT_FOUND,
            AppError::RunNotFound => StatusCode::NOT_FOUND,
            AppError::ParticipantNotFound => StatusCode::NOT_FOUND,
            AppError::BeforePostNotFound => StatusCode::NOT_FOUND,
            AppError::AfterPostNotFound => StatusCode::NOT_FOUND,
            // BadRequest typed variants
            AppError::InvalidPostId => StatusCode::BAD_REQUEST,
            AppError::InvalidChannelId => StatusCode::BAD_REQUEST,
            AppError::ChannelAlreadyArchived => StatusCode::BAD_REQUEST,
            AppError::ChannelNotArchived => StatusCode::BAD_REQUEST,
            AppError::InvalidUserId => StatusCode::BAD_REQUEST,
            AppError::InvalidTeamId => StatusCode::BAD_REQUEST,
            AppError::InvalidId => StatusCode::BAD_REQUEST,
            AppError::InvalidCursor => StatusCode::BAD_REQUEST,
            AppError::InvalidRootId => StatusCode::BAD_REQUEST,
            AppError::InvalidGroupId => StatusCode::BAD_REQUEST,
            AppError::InvalidFileId => StatusCode::BAD_REQUEST,
            AppError::InvalidThreadId => StatusCode::BAD_REQUEST,
            AppError::InvalidSessionId => StatusCode::BAD_REQUEST,
            AppError::InvalidBookmarkId => StatusCode::BAD_REQUEST,
            AppError::InvalidEmojiId => StatusCode::BAD_REQUEST,
            AppError::InvalidBotId => StatusCode::BAD_REQUEST,
            AppError::InvalidWebhookId => StatusCode::BAD_REQUEST,
            AppError::InvalidCommandId => StatusCode::BAD_REQUEST,
            AppError::InvalidSchemeId => StatusCode::BAD_REQUEST,
            AppError::InvalidRoleId => StatusCode::BAD_REQUEST,
            AppError::InvalidUploadId => StatusCode::BAD_REQUEST,
            AppError::InvalidPluginId => StatusCode::BAD_REQUEST,
            // Validation typed variants
            AppError::ValidationInvalidPostId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidChannelId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidUserId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidTeamId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidRootId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidScheduledAt => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidScheduledPostId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidTargetAt => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidGroupId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidFileId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidThreadId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidSessionId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidBookmarkId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidEmojiId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidBotId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidWebhookId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidCommandId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidSchemeId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidRoleId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidUploadId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidPluginId => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::ValidationInvalidHookId => StatusCode::UNPROCESSABLE_ENTITY,
            // Forbidden typed variants
            AppError::NotAMember => StatusCode::FORBIDDEN,
            AppError::NotOnTeam => StatusCode::FORBIDDEN,
            AppError::AdminRequired => StatusCode::FORBIDDEN,
            AppError::InsufficientPermissions => StatusCode::FORBIDDEN,
            AppError::CannotAccessOthersPosts => StatusCode::FORBIDDEN,
            AppError::CannotEditOthersPosts => StatusCode::FORBIDDEN,
            AppError::CannotDeletePost => StatusCode::FORBIDDEN,
            AppError::CannotDeleteOthersPosts => StatusCode::FORBIDDEN,
            AppError::CannotUpdateOthersScheduledPost => StatusCode::FORBIDDEN,
            AppError::CannotDeleteOthersScheduledPost => StatusCode::FORBIDDEN,
            AppError::CannotAcknowledgeForOther => StatusCode::FORBIDDEN,
            AppError::NotInCall => StatusCode::FORBIDDEN,
            AppError::CannotAccessBot => StatusCode::FORBIDDEN,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.to_string();

        // Log the error for debugging: 5xx is error, 4xx is warn
        if status.is_server_error() {
            tracing::error!(error = %message, code = %self.code(), status = %status, "API error");
        } else {
            tracing::warn!(error = %message, code = %self.code(), status = %status, "API error");
        }

        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code().to_string(),
                message,
                details: None,
            },
        };

        (status, Json(body)).into_response()
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Internal(format!("Serialization error: {}", err))
    }
}

impl From<crate::services::llm::LlmError> for AppError {
    fn from(err: crate::services::llm::LlmError) -> Self {
        AppError::ExternalService(err.to_string())
    }
}

/// Result type alias for API handlers
pub type ApiResult<T> = Result<T, AppError>;
