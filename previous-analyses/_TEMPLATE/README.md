# Compatibility Analysis Template

This directory contains the standard template for RustChat compatibility analyses.

## When to Use This Template

Use this template whenever you are analyzing upstream Mattermost behavior for a change that affects:

- API v4 route contracts (status codes, response shapes, headers)
- WebSocket event names or payloads
- Pagination, ordering, or filtering defaults
- Calls plugin behavior or signaling
- Mobile client expectations

## Workflow

1. **Create a new analysis folder** under `previous-analyses/` with the naming convention:
   ```
   previous-analyses/YYYY-MM-DD-<topic>/
   ```
   Example: `previous-analyses/2026-04-27-post-pagination-gaps/`

2. **Copy this template** into the new folder as `ANALYSIS.md`.

3. **Fill in every section.** Incomplete analyses block implementation. Do not skip the "Test Plan" or "Sign-off" sections.

4. **Reference upstream versions.** Record the exact Mattermost server and mobile commit hashes or version tags you analyzed.

5. **Link the analysis** in your PR description when submitting compatibility-sensitive changes.

## Structure

| File | Purpose |
|------|---------|
| `ANALYSIS.md` | The main analysis document (copy from this template) |
| `screenshots/` | Optional: UI behavior screenshots or API response diffs |
| `traces/` | Optional: Wireshark, browser HAR, or `curl` output dumps |
| `notes.md` | Optional: Raw research notes, scratch work, or references |

## Quality Standards

- Every claim about upstream behavior must be reproducible.
- Include exact HTTP requests (method, path, body) and responses (status, headers, body).
- WebSocket events must include the full JSON payload.
- Gaps must be explicit: "RustChat returns X; upstream returns Y."
- Recommended fixes must be testable.

## Sign-off

The analysis is not complete until the sign-off section is filled. At minimum, one reviewer must confirm that the upstream behavior is correctly captured and the gap analysis is accurate.
