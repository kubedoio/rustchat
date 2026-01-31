# RustChat vs Mattermost (HEAD) – Delta Report

Scope: RustChat v4 API implementation vs Mattermost server (../mattermost at HEAD). Focused on API behavior and backend logic parity, with emphasis on posts-related endpoints.

## High-Level Delta

- Endpoint coverage: 390/418 implemented in RustChat (~93%); 57 endpoints still missing per `docs/mattermost-v4-comparison.md`.
- Behavior parity: medium-to-low. Many endpoints are minimal or compatibility stubs, and several behaviors differ materially from Mattermost (permissions, auditing, feature flags, licensing, and data processing).
- Overall difference: substantial. RustChat provides a compatible surface, but implementation details and guarantees differ across most complex endpoints.

## Cross-Cutting Differences

- Permissions: Mattermost enforces granular permissions and role checks (e.g., `PermissionEditOtherUsers`, `PermissionReadChannelContent`, `PermissionManageSystem`). RustChat often uses simple membership checks or admin role checks and omits many nuanced permission gates.
- Licensing & feature flags: Mattermost gates multiple features (acknowledgements, move thread, burn/reveal) behind licenses or flags. RustChat generally does not implement those gates and returns success or minimal behavior.
- Auditing/metrics: Mattermost logs audit events and records metrics for key APIs (search, edits, reveal/burn). RustChat does not include audit/metrics pathways.
- Metadata sanitization: Mattermost sanitizes post metadata and embeds for the requesting user and for preview posts; RustChat returns raw data with minimal filtering.
- ETags and caching: Mattermost sets ETags on post list responses. RustChat does not provide caching headers or ETag logic.

## Posts API – Detailed Gaps

### Search

- Mattermost: full search pipeline with `SearchPostsForUser` (permissions, team filtering, OR searches, deleted channel handling, timezone offsets, metrics). Returns `PostListWithSearchMatches` with match metadata and can search all teams.
- RustChat: simple `ILIKE` query with channel membership check; ignores OR search, timezone offset, and deleted channel handling. Match data is returned as empty arrays. The new `/api/v4/posts/search` uses the same simplified query.

### Posts Around Unread

- Mattermost: uses `GetPostsForChannelAroundLastUnread`, supports thread collapsing flags, and backfills with `GetPostsPage` when no unread state exists; includes ETags and metadata sanitization.
- RustChat: uses `channel_reads.last_read_message_id` and a union query for before/after posts. No thread collapsing behavior, no ETags, and no additional backfill logic.

### Acknowledgements

- Mattermost: license-gated (professional), uses `SaveAcknowledgementForPost` and `DeleteAcknowledgementForPost`, requires read permissions on the post.
- RustChat: no license gate; stores acknowledgements in `post_acknowledgements`. Deletes are blocked after 5 minutes, which does not exist in Mattermost behavior.

### Create / Update / Patch / Delete

- Mattermost: extensive validation and permission checks (edit time limits, edit others’ posts, file upload permissions on update), sanitizes input and metadata, and handles burn-on-read reveal flows.
- RustChat: minimal validation. Edits and deletes generally require only author match; file permissions and edit time limits are not enforced. Post metadata and embeds are not sanitized.

### Pin / Unpin

- Mattermost: uses post patching with permission checks and audit events.
- RustChat: direct `is_pinned` update with membership check only; no audit or metadata filtering.

### Move Thread

- Mattermost: feature-flagged and license-gated; checks wrangler roles and membership; uses app-level move flow.
- RustChat: stub that validates membership only and returns status OK without moving threads.

### Reveal / Burn (Burn-on-read)

- Mattermost: feature-flagged, checks membership, blocks revealing own posts, and uses app-level reveal/burn logic.
- RustChat: simple membership check and returns OK; does not implement actual burn-on-read lifecycle or feature gating.

### Reactions

- Mattermost: permission checks, validation, metadata updates, and audit handling.
- RustChat: validates emoji names and inserts reactions but does not check channel read permissions before reacting; no audit handling.

## Data Model & Storage Differences

- Mattermost uses a layered app/store architecture with extensive caches and plugins. RustChat uses direct SQLx queries and a simpler schema.
- RustChat’s schema uses `channel_reads.last_read_message_id` and a monotonic `posts.seq`, but omits many server-side structures Mattermost relies on (e.g., full post metadata tables, advanced search indexes, audit tables, policy tables).

## Summary Judgment

RustChat achieves strong endpoint surface coverage but is materially different from Mattermost in behavior, security checks, licensing/feature gating, and metadata handling. For basic client compatibility it performs well, but for production parity—especially around permissions, search behavior, audit/metrics, and advanced post features—there are significant gaps.
