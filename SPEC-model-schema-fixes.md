# SPEC: Fix Model-Schema Mismatches in Backend

## Problem Statement

Three independent model-schema mismatches in `backend/src/models/` cause SQLx runtime failures or latent bugs when queries map result rows to Rust structs.

---

## 1. Scope

### Mismatch A: `TeamMember` missing `presence`

**Schema:** `team_members.presence VARCHAR(20) NOT NULL DEFAULT 'offline'` (migration `20260402112243`)

**Changes:**
1. Add `presence: String` to `models::team::TeamMember`.
2. Update `api/v4/teams/mod.rs` — `map_team_member()` to read `member.presence` instead of hard-coding `None`.
3. Update `mattermost_compat/mappers.rs` — `From<TeamMember> for mm::TeamMember` to read `m.presence` instead of hard-coding `None`.
4. Update `api/v4/teams/members.rs::get_team_member_me()` (line ~196) to use `map_team_member(member)` instead of manually constructing `mm::TeamMember` (or at least populate `presence` from the struct).

**Queries affected (all use `SELECT/RETURNING *` → `TeamMember`):**
- `api/teams.rs:207` — `ensure_team_management_access`
- `api/teams.rs:437` — `join_team`
- `api/teams.rs:303` — `add_team_member`
- `api/teams.rs:448` — `join_team` (RETURNING *)
- `api/admin.rs:1781` — `add_team_member` (RETURNING *)
- `api/v4/posts/search.rs:56` — team membership check
- `api/v4/teams/members.rs:135` — `get_team_member`
- `api/v4/teams/members.rs:188` — `get_team_member_me`
- `api/v4/teams/members.rs:279` — `update_team_member_scheme_roles`

### Mismatch B: `Post` missing `has_reactions`

**Schema:** `posts.has_reactions BOOLEAN DEFAULT FALSE` (migration `20260222000002`)

**Changes:**
1. Add `#[sqlx(default)] has_reactions: bool` to `models::post::Post`.
2. No query currently selects `has_reactions` mapped to `Post`, so this is a defensive alignment.

### Mismatch C: `models::post::Reaction` is wrong for the `reactions` table

**Correct schema:** `reactions(id UUID, post_id UUID, user_id UUID, emoji_name VARCHAR(64), create_at BIGINT)`

**Wrong struct:** `models::post::Reaction` — missing `id`, field named `created_at` with type `DateTime<Utc>` instead of `create_at: i64`.

**Changes:**
1. **Remove** `Reaction` from `models::post.rs`.
2. **Use** `models::reaction::Reaction` (correct struct in `models/reaction.rs`) in all call sites.
3. **Fix SQL column references** from `created_at` to `create_at`:
   - `api/v4/posts/reactions.rs:318` — `get_reactions` query
   - `api/v4/posts/reactions.rs:175` — `reactions_for_posts` query
   - `api/posts.rs:807` — `populate_reactions` query (`ORDER BY create_at`)
4. **Fix timestamp arithmetic** in `api/v4/posts/reactions.rs` — replace `reaction.created_at.timestamp_millis()` with `reaction.create_at` (already `i64`).
5. **Fix websocket deserialisation** in `api/v4/websocket.rs` (~lines 1385, 1412) — change `models::post::Reaction` to `models::reaction::Reaction`. The inbound JSON payload for websocket events contains `create_at: i64`, so `models::reaction::Reaction` deserialises correctly.

> **Naming collision note:** `models::post::ReactionResponse` (aggregated emoji→count+users) must remain in `post.rs`; do NOT add `pub use reaction::*` to `models/mod.rs` because `reaction.rs` also defines a `ReactionResponse` (Mattermost-compat DTO with different fields). All references to the DB `Reaction` model must use the explicit path `crate::models::reaction::Reaction`.

---

## 2. Contract Impact

| Surface | Impact | Detail |
|---------|--------|--------|
| `mm::TeamMember` responses | **Minor additive** | `presence` field will now contain the DB value (`"offline"`, `"online"`, etc.) instead of being omitted/ `None`. This is additive and matches Mattermost mobile expectations. |
| `Post` (native API) | None | `has_reactions` is not yet emitted in any response; this change only prevents future runtime failures. |
| `mm::Reaction` responses | None | Reaction endpoints already construct `mm::Reaction` manually; the fix only corrects the internal mapping layer. |
| WebSocket `reaction_added` / `reaction_removed` | **Fix** | Currently the websocket layer deserialises into the wrong struct (`created_at: DateTime<Utc>`). After the fix it deserialises into the correct struct (`create_at: i64`) and produces the same `mm::Reaction` payload. |
| `post.schema.json` | None | Schema already documents `has_reactions: boolean`; model now aligns. |

---

## 3. Risk Assessment

| Mismatch | Breaking? | Risk Level | Test Requirements |
|----------|-----------|------------|-------------------|
| A — `TeamMember.presence` | Non-breaking (additive field on internal struct; API response gains real data) | Low | Unit test for `TeamMember` `FromRow` with `presence`. Integration tests for `GET /api/v4/teams/{id}/members/{user_id}` and `GET /api/v4/users/me/teams/members`. |
| B — `Post.has_reactions` | Non-breaking (defensive, unused in queries today) | Very Low | Compile-time check is sufficient; no runtime path exercises this yet. |
| C — `Reaction` model swap | Potentially breaking if any caller relied on `models::post::Reaction` shape | Medium | Unit test for `models::reaction::Reaction` `FromRow` against `reactions` table. Integration tests for:<br>• `POST /api/v4/posts/{id}/reactions`<br>• `DELETE /api/v4/posts/{id}/reactions/{emoji_name}`<br>• `GET /api/v4/posts/{id}/reactions`<br>• WebSocket broadcast of `reaction_added` / `reaction_removed` |

**Additional latent bugs fixed as part of C:**
- `api/v4/posts/reactions.rs` queries selecting `r.created_at` (column does not exist → SQLx runtime error).
- `api/posts.rs:807` query ordering by `created_at` (column does not exist → SQLx runtime error).
- `api/posts.rs:806` query mapping `SELECT * FROM reactions` to `Vec<Reaction>` where `Reaction` resolved to `models::post::Reaction` (wrong shape → SQLx runtime error).

These latent bugs suggest the reaction code paths are either untested or the errors are silently swallowed. After the fix, the paths will actually execute.

---

## 4. Verification Criteria

### Compilation
```bash
cd backend
cargo check
cargo clippy --all-targets --all-features -- -D warnings
```
Must pass clean.

### Integration Tests
```bash
cd backend
cargo test --no-fail-fast -- --nocapture
```
All existing tests must pass; no new SQLx runtime errors may appear.

### Manual / Smoke Checks
1. **Team member presence:**
   ```bash
   curl -s -H "Authorization: Bearer $TOKEN" \
     "$BASE/api/v4/teams/$TEAM_ID/members/$USER_ID" | jq '.presence'
   ```
   Should return `"offline"` (or actual presence) instead of `null`.

2. **Reactions round-trip:**
   ```bash
   # Add reaction
   curl -s -X POST -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"user_id":"...","post_id":"...","emoji_name":"+1"}' \
     "$BASE/api/v4/posts/$POST_ID/reactions"

   # List reactions
   curl -s -H "Authorization: Bearer $TOKEN" \
     "$BASE/api/v4/posts/$POST_ID/reactions" | jq '.[0].create_at'
   ```
   Should return a valid millisecond timestamp integer, not a 500 error.

3. **WebSocket reaction events:** Connect via WebSocket, add a reaction, verify `reaction_added` event is broadcast with `create_at` as integer.

---

## 5. Rollback Plan

1. **Database:** No migrations required; all schema changes are already applied. Rolling back code only affects model-to-row mapping, not the database.
2. **Code revert:** The changes are confined to:
   - `backend/src/models/team.rs`
   - `backend/src/models/post.rs`
   - `backend/src/models/mod.rs` (if `pub use reaction::*` is added — **do not add it**)
   - `backend/src/api/v4/teams/mod.rs`
   - `backend/src/mattermost_compat/mappers.rs`
   - `backend/src/api/v4/teams/members.rs`
   - `backend/src/api/v4/posts/reactions.rs`
   - `backend/src/api/posts.rs`
   - `backend/src/api/v4/websocket.rs`

   A single `git revert` of the implementation commit restores the previous (broken) state.
3. **Operational:** If a runtime SQLx error surfaces in production after deploy, the query-level failure will return 500. Monitor error rates on:
   - `POST /api/v4/posts/*/reactions`
   - `DELETE /api/v4/posts/*/reactions/*`
   - `GET /api/v4/teams/*/members/*`
   Roll back immediately if error rate exceeds baseline.

---

## Appendix: Files to Modify

| File | Lines of Interest | Change |
|------|-------------------|--------|
| `backend/src/models/team.rs` | 31-37 | Add `presence: String` to `TeamMember` |
| `backend/src/models/post.rs` | 30-36 | Remove `Reaction` struct; keep `ReactionResponse` |
| `backend/src/models/post.rs` | 9-28 | Add `#[sqlx(default)] has_reactions: bool` to `Post` |
| `backend/src/api/v4/teams/mod.rs` | 98-111 | `map_team_member` — use `member.presence` |
| `backend/src/api/v4/teams/members.rs` | 196-204 | `get_team_member_me` — use `map_team_member` or populate `presence` |
| `backend/src/mattermost_compat/mappers.rs` | 274-287 | `From<TeamMember>` — use `m.presence` |
| `backend/src/api/v4/posts/reactions.rs` | 94-106 | Use `crate::models::reaction::Reaction`; fix timestamp |
| `backend/src/api/v4/posts/reactions.rs` | 138-147 | Use `reaction.create_at` instead of `.timestamp_millis()` |
| `backend/src/api/v4/posts/reactions.rs` | 173-183 | Fix `r.created_at` → `r.create_at` in query |
| `backend/src/api/v4/posts/reactions.rs` | 255-262 | Use `crate::models::reaction::Reaction` |
| `backend/src/api/v4/posts/reactions.rs` | 279-288 | Fix `r.created_at` → `r.create_at` in query and timestamp handling |
| `backend/src/api/posts.rs` | 806-810 | Use `crate::models::reaction::Reaction`; fix `ORDER BY create_at` |
| `backend/src/api/v4/websocket.rs` | 1383-1408 | Use `crate::models::reaction::Reaction`; fix `create_at` handling |
| `backend/src/api/v4/websocket.rs` | 1410-1425 | Use `crate::models::reaction::Reaction`; fix `create_at` handling |
