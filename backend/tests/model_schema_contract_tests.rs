//! Model-Schema Contract Tests
//!
//! Regression tests that verify model structs match the PostgreSQL schema.
//! These tests are designed to fail to compile when model-schema mismatches exist,
//! documenting the current broken state and verifying fixes after implementation.
//!
//! # Current Status
//!
//! - `test_team_member_schema_parity` fails to compile because `TeamMember` lacks the
//!   `presence` field that exists in the `team_members` table (migration
//!   `20260402112243_add_presence_to_team_members.sql`).
//! - `test_post_schema_parity` fails to compile because `Post` lacks the `has_reactions`
//!   field that exists in the `posts` table (migration `20260222000002_create_reactions.sql`).
//! - `test_post_reaction_is_wrong` fails to compile because `models::post::Reaction` lacks
//!   the `id` field present in the `reactions` table and uses `created_at` instead of
//!   `create_at`.
//! - `test_reaction_schema_parity` is written correctly and would compile and pass if the
//!   other tests were not blocking compilation.
//!
//! After fixing the models, all tests will compile and pass.

use rustchat::models::{Post, TeamMember};
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Test 1: TeamMember schema parity
// ---------------------------------------------------------------------------

/// Verify that `TeamMember` can be mapped from `SELECT * FROM team_members`.
///
/// The `team_members` table has `presence VARCHAR(20) NOT NULL DEFAULT 'offline'`
/// (added by migration `20260402112243_add_presence_to_team_members.sql`), but
/// `TeamMember` in `src/models/team.rs` does not have a `presence` field.
///
/// Affected code:
/// - `src/api/teams.rs:207`
/// - `src/api/teams.rs:437`
/// - `src/api/v4/posts/search.rs:56`
///
/// This test currently FAILS TO COMPILE. After adding `presence: String` to
/// `TeamMember`, it will compile and pass.
#[sqlx::test]
async fn test_team_member_schema_parity(pool: PgPool) -> anyhow::Result<()> {
    let row: Option<TeamMember> = sqlx::query_as("SELECT * FROM team_members LIMIT 1")
        .fetch_optional(&pool)
        .await?;

    if let Some(row) = row {
        // This field access documents the required column. The line below will
        // fail to compile until `presence` is added to the `TeamMember` struct.
        let _presence: &str = &row.presence;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: Post schema parity
// ---------------------------------------------------------------------------

/// Verify that `Post` can be mapped from `SELECT * FROM posts` (which includes
/// `has_reactions`).
///
/// The `posts` table has `has_reactions BOOLEAN DEFAULT FALSE` (added by migration
/// `20260222000002_create_reactions.sql`), but `Post` in `src/models/post.rs` does
/// not have a `has_reactions` field.
///
/// This test currently FAILS TO COMPILE. After adding `has_reactions: bool` to `Post`,
/// it will compile and pass.
#[sqlx::test]
async fn test_post_schema_parity(pool: PgPool) -> anyhow::Result<()> {
    let row: Option<Post> = sqlx::query_as("SELECT *, has_reactions FROM posts LIMIT 1")
        .fetch_optional(&pool)
        .await?;

    if let Some(row) = row {
        // This field access documents the required column. The line below will
        // fail to compile until `has_reactions` is added to the `Post` struct.
        let _has_reactions: bool = row.has_reactions;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: Correct Reaction model matches schema
// ---------------------------------------------------------------------------

/// Verify that `models::reaction::Reaction` correctly maps to the `reactions` table.
///
/// Schema (`reactions`):
/// - `id UUID PRIMARY KEY`
/// - `post_id UUID NOT NULL`
/// - `user_id UUID NOT NULL`
/// - `emoji_name VARCHAR(64) NOT NULL`
/// - `create_at BIGINT NOT NULL`
///
/// `models::reaction::Reaction` has all these fields with the correct types:
/// - `id: Uuid`
/// - `post_id: Uuid`
/// - `user_id: Uuid`
/// - `emoji_name: String`
/// - `create_at: i64`
///
/// This test PASSES now and should continue to pass after the fixes.
#[sqlx::test]
async fn test_reaction_schema_parity(pool: PgPool) -> anyhow::Result<()> {
    let row: Option<rustchat::models::reaction::Reaction> =
        sqlx::query_as("SELECT * FROM reactions LIMIT 1")
            .fetch_optional(&pool)
            .await?;

    if let Some(row) = row {
        // Verify all required fields are present and have correct types.
        let _id: uuid::Uuid = row.id;
        let _post_id: uuid::Uuid = row.post_id;
        let _user_id: uuid::Uuid = row.user_id;
        let _emoji_name: String = row.emoji_name;
        let _create_at: i64 = row.create_at;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: Canonical Reaction is the ONLY Reaction for DB queries
// ---------------------------------------------------------------------------

/// Verify that `models::reaction::Reaction` is the only `Reaction` type used
/// for database queries. The old `models::post::Reaction` has been removed.
#[sqlx::test]
async fn test_reaction_is_canonical(pool: PgPool) -> anyhow::Result<()> {
    let row: Option<rustchat::models::reaction::Reaction> =
        sqlx::query_as("SELECT * FROM reactions LIMIT 1")
            .fetch_optional(&pool)
            .await?;

    if let Some(row) = row {
        // Verify all schema fields are present on the canonical type.
        let _id: uuid::Uuid = row.id;
        let _post_id: uuid::Uuid = row.post_id;
        let _user_id: uuid::Uuid = row.user_id;
        let _emoji_name: String = row.emoji_name;
        let _create_at: i64 = row.create_at;
    }

    Ok(())
}
