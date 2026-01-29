# Mattermost Mobile Knowhow (Rustchat)

This note captures key behaviors and integration expectations discovered while debugging the Mattermost mobile app against Rustchat.

## File Upload Flow (Mobile)

- **Endpoint:** `POST /api/v4/files`
- **Multipart field name:** `files`
- **Multipart form fields:**
  - `channel_id`: the channel ID (Mattermost base32 ID)
- **Behavior:** the mobile client will **not** send any network request if the file object lacks a `localPath`.
  - In that case, the mobile app logs: `error on uploadFile` with `file does not have local path defined`.

### Implication

If you see **no** `/api/v4/files` request in server logs, the file picker likely returned a URI that the client cannot resolve to a local path. Fixing this is on the client side (picker permissions/URI handling), not the server.

## Posting Attachments

- Mobile posts are sent to `POST /api/v4/posts` with `file_ids` set.
- If `file_ids` is empty **and** `message` is empty, Rustchat returns `422` (Validation error: message cannot be empty).

## File Preview Endpoints (Required by Mobile)

The mobile app renders images using these endpoints:

- `GET /api/v4/files/{id}`
- `GET /api/v4/files/{id}/thumbnail`
- **`GET /api/v4/files/{id}/preview`** (required)

If `/preview` is missing, images may upload but not display on mobile.

## Presigned URL Host (Public Endpoint)

Presigned URLs must be generated with the **public** host that the mobile client can reach.
Rewriting the host after signing causes `SignatureDoesNotMatch`.

### Config

Set a public endpoint for presigning:

```
RUSTCHAT_S3_PUBLIC_ENDPOINT=https://<public-s3-host>
```

## S3 Bucket Consistency

Ensure the backend bucket matches the bucket created by the provisioning step.

- If `createbuckets` creates `rustchat-uploads`, the backend **must** use:

```
RUSTCHAT_S3_BUCKET=rustchat-uploads
```

## Debugging Checklist

1. **No `/api/v4/files` logs**
   - Check mobile logcat for `error on uploadFile`.
   - Likely `localPath` missing.

2. **`POST /api/v4/posts` returns 422**
   - Confirm `file_ids` is set in the post payload.

3. **Uploads succeed, but images don’t show**
   - Ensure `/api/v4/files/{id}/preview` exists.
   - Ensure presigned URL uses public host.

4. **SignatureDoesNotMatch**
   - Presign using the public endpoint (do not rewrite the host).

## Useful Log Filters

Server side:

```
docker compose logs -f rustchat-frontend | egrep "api/v4/files|api/v4/posts"
docker compose logs -f rustchat-backend
```

Mobile (Android):

```
adb logcat | grep -i -e "upload" -e "file" -e "network" -e "okhttp"
```
