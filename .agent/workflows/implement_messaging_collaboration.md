---
description: Workflow to implement shared messaging workspace features (Threads, Files, Pins)
---

# Implement Messaging Collaboration

This workflow guides the implementation of core messaging features required for Mattermost parity.

## Prerequisite Check
- [x] Backend Post CRUD
- [x] Backend File Upload
- [x] Backend Threading Support (`root_post_id`)

## Phase 1: Thread Support (Frontend)
1. Create `ThreadView.vue` sidebar component.
2. Update `PostItem.vue` to show "Reply" button and reply count.
3. Implement `useThreads` composable to fetch thread details.
4. Integrate `ThreadView` into the `ChannelLayout`.

## Phase 2: File Attachments (Frontend)
1. Create `FileUploader.vue` component (Drag & Drop zone).
2. Integrate with `POST /api/v1/files`.
3. Update `MessageComposer` to support attaching files.
4. Create `FilePreview` component for `PostItem`.

## Phase 3: Message Actions (Frontend)
1. Create `MessageActions` dropdown menu component.
2. Implement **Edit** action (switch PostItem to edit mode).
3. Implement **Delete** action (confirm modal).
4. Implement **Pin/Unpin** action.

## Phase 4: Saved Messages (Full Stack)
1. **Backend**: Create migration for `saved_posts` table (user_id, post_id).
2. **Backend**: Add `POST /posts/{id}/save` and `DELETE /posts/{id}/save` endpoints.
3. **Backend**: Add `GET /users/me/saved` endpoint.
4. **Frontend**: Add "Save" action to Message Menu.
5. **Frontend**: Create `SavedPostsView` sidebar.

## Phase 5: Rich Formatting
1. Integrate `markdown-it` or similar library for message rendering.
2. Add Emoji Picker (e.g., `emoji-mart-vue`).
3. Add Syntax Highlighting for code blocks.
