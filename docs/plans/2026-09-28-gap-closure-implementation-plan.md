# Gap Closure Implementation Plan — Fast Wins First

**Date:** 2026-09-28  
**Status:** Proposed  
**Scope:** close the gaps identified by the 2026-09-28 maturity/production-readiness analysis, starting with the fast, low-risk items, then medium and large items in dependency order  
**Baseline:** `fix/review-round8` @ `dabacc3` (source v0.5.1)

## 1. Goal

Convert the known production-readiness gaps from the current-state/roadmap lists into
closed, verified work — smallest and safest first — without weakening any existing
gate (RI-C01…C10), and while respecting governance boundaries
(`.governance/agent-contracts.yml`, `.governance/pr-size-limits.yml`,
`.governance/risk-tiers.yml`).

Every Tier-1 item below was verified against the baseline tree before planning
(file:line references are from `dabacc3`). Items that turned out to be already done or
misreported were dropped, not planned (see §6).

## 2. Triage rules

- **Tier 1 (fast/simple):** ≤ 1 day each, ≤ 10 files / ≤ 300 lines per PR, standard
  risk, no gated paths, testable with existing check commands.
- **Tier 2 (medium):** days, may touch elevated-risk areas (permissions, realtime),
  need bounded slices and regression tests.
- **Tier 3 (large):** weeks or design-first; require their own spec (and where
  architectural, an ADR).
- **Critical path:** the pending v0.5.1 release (issue #259) is blocked by #88 and
  #89 — so the #88/#89 triage (PR-5) is the highest-priority Tier-1 item even though
  the eventual fix size is unknown.

## 3. Tier 1 — fast, simple, verified

Each item is one independently mergeable PR. Conventional Commits + DCO sign-off
required. All are **standard risk**; none touch a protected path.

### PR-1 — Documentation truthfulness sweep

**Objective:** make the top-level contributor docs match the tree.

Changes:

1. `AGENTS.md` — remove the stale `backend/src/a2a` references (the module no longer
   exists):
   - line ~7 (backend `src/` module listing includes `a2a`);
   - line 87 (agent-boundary table: `backend/src/a2a/**` (senior review)).
2. `ROADMAP.md` — mark **"Rate limiting coverage — File upload and search
   endpoints"** as complete, with evidence pointers: upload, search, and WebSocket
   IP rate limits are wired (`backend/src/api/search.rs:21`,
   `backend/src/api/v4/mod.rs:67,135`) and covered by
   `backend/tests/test_rate_limiting.rs` (13 tests).
3. `README.md` — add the missing GitHub Actions CI status badge for `ci.yml`
   (only license/Rust/Vue badges exist today).

**Deliberately out of scope:** `a2a` also appears in `.governance/protected-paths.yml`
and `.governance/agent-contracts.yml`. Those are protected paths (architectal tier);
cleaning them requires a separately justified governance PR (see Tier 2, M7). Do not
bundle them here.

- Files: 3 (`AGENTS.md`, `ROADMAP.md`, `README.md`) — within the 600-line docs limit.
- Validation: `docs` CI job (link check, hygiene); manual re-read of diff.
- Acceptance: `rg -n "src/a2a" AGENTS.md` returns nothing; ROADMAP no longer lists
  rate-limit coverage as open.

### PR-2 — Frontend dead-code removal

**Objective:** remove leftover scaffolding and the duplicate thread panel.

Verified state:

- `frontend/tests/example.spec.ts` is a leftover Vite/Playwright scaffold (a skipped
  login flow targeting a hardcoded `localhost:3000`); it is not executed by
  `vitest run src/` and does not match the real Playwright layout under `frontend/e2e/`.
- `frontend/src/components/thread/ThreadPanel.vue` is dead: the live panel is
  `components/channel/ThreadPanel.vue`, rendered via
  `src/components/layout/RightSidebar.vue:5`; the only reference to the `thread/`
  copy is an unused barrel re-export at `src/features/messages/index.ts:30`.

Changes:

1. Delete `frontend/tests/example.spec.ts` (and `frontend/tests/` if empty).
2. Delete `frontend/src/components/thread/ThreadPanel.vue`.
3. Remove the re-export line `src/features/messages/index.ts:30`.
4. Diff the two files first; if `thread/ThreadPanel.vue` contains anything the live
   one lacks, port it instead of deleting — otherwise delete.

- Files: 3 (2 deletions + 1 edit).
- Validation: `npm run lint`, `npm run test:unit`, `npm run build` (vue-tsc catches
  dangling imports).
- Acceptance: `rg -rn "components/thread/ThreadPanel" frontend/src` empty; build green.

### PR-3 — Backend micro-fixes (unwrap hygiene)

**Objective:** remove the two known production-code `unwrap()` smells.

Changes:

1. `backend/src/services/knowledge/hybrid_search.rs:60` — replace
   `b.fused_score.partial_cmp(&a.fused_score).unwrap()` with
   `b.fused_score.total_cmp(&a.fused_score)` (stable since Rust 1.62; handles NaN
   deterministically). Add a unit test with a NaN score in the result set.
2. `backend/src/middleware/security_headers.rs` (~lines 181–234 per the production-gap
   audit) — startup-time `.parse().unwrap()` calls on config values. Re-read the
   current code first; replace each with a `ConfigError`/`AppError` that fails fast
   with the offending value and variable name (misconfigured security headers should
   abort startup loudly, not panic). Add a unit test for the invalid-value path.

- Files: 2 + tests in the same files (all within `#[cfg(test)]` modules).
- Validation: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --lib`.
- Acceptance: no `.unwrap()` remains in either file outside `#[cfg(test)]`; new tests
  pass.

### PR-4 — SVG upload screening hardening

**Objective:** close the weak SVG filter flagged as GAP-23.

Current state (verified): `backend/src/api/file_validation.rs:232-235` rejects only
`<script`. An uploaded SVG can still carry `onload=`-style event handler attributes,
`javascript:`/`data:` URLs in `href`/`xlink:href`, `<foreignObject>` HTML embeds, and
external references.

Changes (same file, plus tests):

1. Reject SVG markup containing: `on\w+=` event-handler attributes,
   `javascript:` and `data:` URI schemes in any attribute value, `<foreignObject`,
   `<use href="#…"`/external `xlink:href` references to remote resources, and
   `<!DOCTYPE`/`<!ENTITY` (XXE/billion-laughs vector).
2. Keep the existing `<script` rejection.
3. Add unit tests for each bypass vector (pattern the existing test at line 294).

Note: the durable fix is serving user SVGs with `Content-Disposition: attachment` +
`Content-Type: image/svg+xml` isolation and/or a sanitizer; screening stays the
upload-time control. If a sanitize-on-upload step is preferred, split it into its own
PR — do not grow this one past the size limit.

- Files: 1–2 (file_validation.rs + its test module).
- Validation: `cargo fmt/clippy -D warnings`, `cargo test --lib`.
- Acceptance: each bypass-vector test fails against the old filter and passes with
  the new one.

### PR-5 — P0 defect triage: #88 (compliance export) and #89 (audit dashboard)

**Objective:** convert two four-month-old P0 bugs from "known broken" into either a
reproduced defect with a failing test, or an explicit, honest de-scope. This is the
release critical path (issue #259 blockers).

Steps (triage only — fixes are Tier 2, sized after reproduction):

1. Reproduce #88: run compliance export against the integration stack
   (`docker-compose.integration.yml`), capture the actual failure mode (error, empty
   output, wrong format).
2. Reproduce #89: seed audit data, compare admin audit-dashboard values against the
   underlying tables; identify the wrong values/access behavior.
3. Write failing integration tests capturing each defect (in
   `backend/tests/`, following existing suite patterns).
4. Decision point, recorded on the issues:
   - **fixable in a bounded slice** → schedule as M1 (Tier 2);
   - **not bounded** → de-scope honestly for v0.5.1: return `501 NOT_IMPLEMENTED`
     from the compliance-export endpoint (matching the existing explicit-stub
     pattern in `api/v4/stubs.rs`), hide/disable the audit-dashboard widgets in the
     admin UI, and update `docs/repo-current-state.md` + README to say so.
5. Update issue #259's checklist with the outcome.

- Files (triage PR): tests only, plus issue/roadmap text edits.
- Validation: new tests reproduce the defects (red) — or, after de-scope, pass
  against the 501 behavior.
- Acceptance: neither bug remains in the "shown but not working" state; #259's
  blockers are resolved or re-scoped.

### Maintenance action (no PR) — close issue #258

Branch/tag protection is implemented and verified (RI-C02,
`docs/github-protection.md`, 2026-09-27). Re-run `scripts/verify-protection.sh`,
attach the output, and close #258 so the open-issue list reflects reality.

## 4. Tier 2 — medium items (dependency order)

| # | Item | Depends on | Notes |
|---|---|---|---|
| M1 | Fix or land the de-scope of #88/#89 | PR-5 | Bounded slices; regression tests mandatory. |
| M2 | Membership-revocation on channel removal (GAP-11) | — | **Elevated risk (permissions).** Unsubscribe WS connections in the realtime hub when a member is removed from a channel; add lifecycle regression tests (`tests/api_v4_websocket_lifecycle.rs` pattern). Maintainer sponsorship per `risk-tiers.yml`. |
| M3 | P006: backup→restore→sanity evidence in CI | — | Extend `scripts/migration-matrix.sh` pattern: pg_dump the integration DB → restore into a fresh container → run readiness + smoke assertions. New workflow job; then check the ROADMAP backup item. |
| M4 | CSP hardening: remove `script-src 'unsafe-inline'` | — | Audit `frontend/index.html` + built bundle for inline scripts; move to hash/nonce or strict `'self'`; keep `style-src` decision separate (Vue inline styles). Verify with full Playwright E2E + manual admin pass. Backend: `middleware/security_headers.rs`. |
| M5 | Gate PRs on backend integration tests | — | Currently only `push: main`/nightly (`integration.yml`). Add a paths-filtered (`backend/**`) PR trigger or a required subset; watch CI cost. |
| M6 | Frontend coverage for the biggest surfaces | — | Bounded slices: `MessageItem.vue`, `ChannelSidebar.vue`, `useWebSocket.ts` unit tests; add a `test:coverage` script with thresholds afterwards, not before. |
| M7 | Governance `.governance/` a2a cleanup + frontend refactor finish | — | (a) Remove stale `a2a` patterns from `.governance/protected-paths.yml` + `agent-contracts.yml` — **protected path, architectural tier, needs justification**. (b) Finish or explicitly abandon the frontend store refactor: migrate `stores/calls.ts` (1,027 lines) into `features/calls/`, delete the unwired `core/websocket/WebSocketManager.ts`, record the outcome in the refactoring notes. |
| M8 | Ship v0.5.1 | M1 (+ #259 checklist) | Execute `docs/release-process.md` end to end: `scripts/check-release-ready.sh`, signed tag, release workflow validation. |

## 5. Tier 3 — large items (design first)

1. **Durable realtime replay/outbox** — the single highest-impact technical gap.
   Needs its own spec (likely an ADR): persist client-facing events with per-connection
   sequence continuity across restarts, bounded retention, replay-on-reconnect, and
   explicit `resync_required` beyond the window. Reuse patterns from the existing
   Buzz durable outbox (`backend/src/integrations/outbox.rs`) where applicable.
2. **`client_msg_id` idempotency + cursor pagination** on hot channel-history paths
   (frontend already sends `crypto.randomUUID()`; backend needs a uniqueness
   constraint + upsert and cursor-based history).
3. **i18n** — no translation layer exists at all; introduce vue-i18n with an
   extraction pass; decide locale policy first (design doc).
4. **RI-C04 debt migration** — 147 direct-SQL call sites in `api/` across 60 files,
   migrated only in bounded, tested slices per the active contract; hotspots first
   (`api/calls.rs`, `api/v1/agents.rs`, `api/admin_stats.rs`).

## 6. Dropped / corrected findings

- "Committed stale `frontend/dist`" from the frontend analysis — **not true at
  baseline** (`git ls-files frontend/dist` is empty); not planned.
- "Rate limiting missing on upload/search" (ROADMAP wording) — middleware exists and
  is wired; planned as a documentation fix (PR-1), not code.
- `model_schema_contract_tests.rs` intentionally-documented drift tests — deliberate
  per the audit; left alone.

## 7. Execution rules (per repo governance)

- Conventional Commits (`fix:`, `docs:`, `chore:`, `test:` …) and DCO `-s` on every
  commit; DCO is enforced in CI.
- PR size limits: code ≤ 10 files / ≤ 300 lines; docs ≤ 600 lines (Tier-1 PRs all fit).
- Tier-1 PRs are standard risk and need no sponsorship; M2 and M7a require
  maintainer sponsorship / architectural justification.
- Validation ladder before opening any backend PR:
  `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib`;
  frontend: `npm run lint && npm run test:unit && npm run build`.
- After each merge: re-check `docs/repo-current-state.md` and `ROADMAP.md` stay
  truthful (RI-C06 hygiene) — the PR that closes a roadmap item updates it.

## 8. Suggested order

```
PR-1 (docs) ─┬─ parallel ─┬─ PR-2 (frontend dead code)
             │            ├─ PR-3 (backend unwraps)
             │            └─ PR-4 (SVG hardening)
PR-5 (triage #88/#89) ──> M1 ──> M8 (release v0.5.1)
close #258 (no PR)
then M2, M3, M4, M5, M6, M7 as capacity allows; Tier 3 by spec.
```
