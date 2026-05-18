//! Repository module for database query patterns
//!
//! Centralizes common SQL queries to reduce duplication across the codebase.

pub mod admin_repository;
pub mod bookmark_repository;
pub mod category_repository;
pub mod channel_repository;
pub mod emoji_repository;
pub mod file_repository;
pub mod group_repository;
pub mod integration_repository;
pub mod oauth_repository;
pub mod playbook_repository;
pub mod post_repository;
pub mod system_repository;
pub mod terms_repository;
pub mod team_repository;
pub mod upload_repository;
pub mod user_repository;

pub use admin_repository::AdminRepository;
pub use bookmark_repository::BookmarkRepository;
pub use category_repository::{CategoryRepository, CategoryRow, SidebarCandidateChannel};
pub use channel_repository::{BookmarkRow, ChannelGroupRow, ChannelRepository, ChannelWithTeamDataRow, ChannelWithTeamDataResponse};
pub use emoji_repository::{DbEmoji, EmojiRepository};
pub use file_repository::FileRepository;
pub use group_repository::GroupRepository;
pub use integration_repository::IntegrationRepository;
pub use oauth_repository::{LegacyProviderRow, OAuthRepository};
pub use playbook_repository::{calculate_progress, PlaybookRepository};
pub use post_repository::{ChannelUnreadStats, PostRepository, PostWithUser, ThreadSnapshotRow};
pub use terms_repository::TermsRepository;
pub use team_repository::TeamRepository;
pub use system_repository::SystemRepository;
pub use upload_repository::{UploadRepository, UploadSessionRow};
pub use user_repository::UserRepository;
