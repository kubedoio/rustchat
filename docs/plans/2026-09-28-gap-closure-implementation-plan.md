# Gap Closure Implementation Plan — Fast Wins First

**Date:** 2026-09-28  
**Status:** Tier 1 executed and merged (2026-09-28/29 — see §3.1); Tier 2 largely executed (M3–M7 delivered, M4 partially — see §4); M2/M8 maintainer-gated; Tier 3 pending.  
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
- **Critical path:** the pending v0.5.1 release (issue #259) was blocked by #88 and
  #89 — both resolved 2026-09-28/29 (see §3.1), so the release (M8) is now the
  critical-path item.

## 3. Tier 1 — fast, simple, verified

### 3.1 Execution record (2026-09-29)

All Tier-1 items are merged. Mapping and deviations from the plan above:

| Plan item | Merged as | Deviations / notes |
|---|---|---|
| PR-1 (docs truthfulness) | #306 | Also fixed two factual errors found in review: the plan's own PR-4 baseline claim and the ROADMAP rate-limiting evidence wording. |
| PR-2 (frontend dead code) | #307 | Scope expanded beyond the duplicate `thread/ThreadPanel.vue`: the whole dead `components/thread/` cluster (4 files) and `composer/ThreadComposer.vue` were verified zero-consumer and deleted (−665 lines, deletion-only; exceeds the 300-line code cap — flagged on the PR, accepted by review). |
| PR-3 (unwrap hygiene) | #308 | `security_headers.rs` panics descriptively at construction instead of returning `ConfigError` (chosen for fail-fast startup); production sort extracted to `sort_hybrid_results()` so the NaN regression test drives real production code. |
| PR-4 (SVG hardening) | #309, #312 | Beyond the plan: closed a whitespace `href =` bypass, a local-fragment `url(#grad)` false positive, and (in #312) a namespaced-element bypass (`<x:script>`). Known limitation (entity encoding) documented in the validator doc comment. |
| PR-5 (triage #88/#89) | #310, #312 | Docker was unavailable, so triage was static code tracing instead of live reproduction. Outcome: #88 honestly de-scoped (phantom endpoint removed, UI labeled "Not implemented"); #89 fixed in two bounded slices — blob export bug (#310) and query serialization 400s (#312) — plus honest relabeling of the dashboard as "Membership Policy Audit". Issues #88/#89 closed with evidence 2026-09-29. |
| Maintenance (#258) | live fix + #311 | Re-verification found the merge gate missing on `main` (empty `required_status_checks`); re-applied the same day, `verify-protection.sh` extended to also assert the review gate (10/10 PASS). #311 records the regression and resolution. |

Follow-up: #313 (open) triages all 30 open CodeQL alerts — three code fixes plus evidence-based dismissals.

Each item below is one independently mergeable PR. Conventional Commits + DCO sign-off
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
and `.governance/agent-contracts.yml`. Those are protected paths (architectural tier);
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

Current state (verified against the baseline): `backend/src/api/file_validation.rs`
already rejects `<script`, six hardcoded event-handler names (`onload`/`onerror`/
`onclick`/`onmouseover`/`onfocus`/`onblur`), `<foreignObject>`, and `href=`/
`xlink:href` attribute forms. Remaining bypass vectors:
the handler allowlist misses lesser-known or future `on*` names and whitespace
variants (`onload =`); active-content URI schemes (`javascript:`, `data:text/html`,
`data:image/svg+xml`) outside `href` values, e.g. in SMIL `values=`; `<style>`
elements; `style` attributes with `url()`/`expression()` fetches; SMIL animations
rewriting `href` at runtime; and `<!ENTITY` declarations (XXE / billion-laughs).
Note that SVG uploads are rejected at the extension layer today, so this validator
is defense-in-depth, not a live path.

Changes (same file, plus tests):

1. Replace the six-name handler allowlist with a general `on[a-z]+\s*=` regex.
2. Reject active-content URI schemes anywhere in the document.
3. Reject `<style` elements and style attributes containing remote `url()` fetches.
4. Reject SMIL animations targeting `href`/`xlink:href`.
5. Reject `<!ENTITY` declarations (plain `<!DOCTYPE svg>` stays allowed).
6. Add unit tests for each bypass vector and for benign patterns that must keep
   passing.

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
| ~~M1~~ | ~~Fix or land the de-scope of #88/#89~~ **DONE** — absorbed by #310 + #312 (see §3.1); issues #88/#89 closed 2026-09-29 | ~~PR-5~~ | — |
| M2 | Membership-revocation on channel removal (GAP-11) | — | **Elevated risk (permissions).** Premise partially stale on re-verification (2026-09-29): v1/v4 manual removal already unsubscribes and is regression-tested; remaining scope is the un-wired removal paths (agents, group sync, team cascade) and cluster-wide revocation. Design note: `docs/plans/2026-09-29-m2-membership-revocation-design.md` — awaiting maintainer sponsorship. |
| M3 | P006: backup→restore→sanity evidence in CI | #316 | Delivered as `scripts/backup-restore-check.sh` + a `Backup Restore` CI job (aggregated into CI Complete): apply all migrations, replicate sqlx bookkeeping, seed 8 core tables, `pg_dump -Fc` → restore into a fresh database → assert semantic schema identity, migration bookkeeping, row counts/checksums, referential integrity. Runbook restore procedure + ROADMAP item closed. |
| M4 | CSP hardening: remove `script-src 'unsafe-inline'` | — | **Partially delivered (2026-09-29, #318):** production SPA policy shipped in Report-Only mode with no `unsafe-inline` in `script-src` (the backend API presets are enforced directly). Remaining (maintainer): manual admin pass against the Report-Only headers, then flip to enforcing in `frontend/nginx.conf`. |
| M5 | Gate PRs on backend integration tests | — | **Delivered (2026-09-29, #319):** paths-filtered (`backend/**`) PR trigger runs the integration suite on backend-touching PRs. |
| M6 | Frontend coverage for the biggest surfaces | — | **Delivered (2026-09-29):** suite-flake root causes fixed first (#321 — misused `vi.waitFor`, leaking `vi.doMock`, order-dependent assertions; flake was pre-existing on main); unit tests for `MessageItem.vue` (38, #322), `ChannelSidebar.vue` (24, #323), `useWebSocket.ts` (22, #324); `test:coverage` script + `@vitest/coverage-v8` with thresholds at honestly measured levels (#328: 38/28/28/39 vs measured 39.07/29.69/29.12/40.02; v8 provider reports test-reachable code only — see PR body). Not wired as a CI gate (separate future decision). Latent realtime bugs found by the new tests are tracked in #320, not silently fixed in test PRs. |
| M7 | Governance `.governance/` a2a cleanup + frontend refactor finish | — | (a) **Delivered (2026-09-29, #335):** stale `a2a` references removed from `.governance/protected-paths.yml`, `.governance/agent-contracts.yml`, `.github/CODEOWNERS`, and their derived/current-state docs (`agent-model.md`, `ownership.md`, architecture docs, admin doc); justification recorded as ADR-007; remaining repo `a2a` hits are Cargo.lock checksum hex and historical point-in-time records. (b) **Delivered (2026-09-29):** dead-code census verified twice, unwired `core/websocket/WebSocketManager.ts` deleted + 5 frontend docs corrected (#325); `stores/calls.ts` migrated to `features/calls/stores/callsStore.ts` as a path-only move (store id `'calls'` + export `useCallsStore` unchanged, Pinia state identity preserved; new `features/calls` barrel; #327, 14-file deviation flagged); `stores/config.ts` then migrated to `features/config/` with the legacy auth-merge guard ported, fixing a live defect where features-store consumers missed `config_updated` updates (#334, closes the store refactor — `stores/` is now empty); outcomes recorded in `frontend/MIGRATION_GUIDE.md`. Current-state docs (refactoring notes, architecture diagram, developer guide, `AGENTS.md` tree, `docs/repo-current-state.md`) re-verified against the tree and corrected (#336). |
| M8 | Ship v0.5.1 | ~~M1~~ **unblocked** (#88/#89/#258 closed; #259 checklist remains) | Execute `docs/release-process.md` end to end: `scripts/check-release-ready.sh`, signed tag, release workflow validation. **Next critical-path item.** |

**Post-plan operational fixes (2026-09-29/30, from the comprehensive review cycle):**
- `fix(deps)`: undici override bumped past newly published GHSA advisories —
  dev-only exposure (production `npm audit --omit=dev` clean), but it failed
  the `npm Audit` CI job on main and every open PR until fixed (#329).
- `fix(ci)`: promotion-gate race fixed (`scripts/promotion-gate.sh` gained an
  opt-in bounded `--wait` for pending required checks) — `Promote Moving
  Aliases` had failed on every recent main commit because it triggered when
  Integration Tests completed while Security (CodeQL) landed ~9 min later,
  freezing the `main`/`nightly` aliases (RI-C03 contract preserved; #330).
  Post-fix gates verified green, but the build stage then exposed a deeper
  pre-existing failure: the arm64-under-QEMU image build exceeds the
  120-minute job timeout (amd64 builds natively in ~26 min), so no promotion
  run had ever completed. Fixed by distributing multi-arch builds across
  native runners (#338). Live validation of the first completed run then
  exposed a third layer: the manifest-merge step raw-interpolated the
  multi-line metadata tag list into its shell script, so the newlines
  split the command substitution and only the first tag (`main`) was
  applied — the run reported green while `nightly`/`nightly-<sha>` were
  never created (and, without `inherit_errexit`, the swallowed inner
  failures never tripped `set -e`). Fixed by passing the tag list through
  the step env and applying the full set in one `imagetools create` call,
  with an empty-tag-set refusal (#340). The post-fix promotion
  (run 36728252070, promoting 7b6e960) completed green end-to-end in
  ~46 minutes (gate → native amd64+arm64 builds → manifest merges
  reporting "Applying 3 tag(s) to 2 platform digest(s)"), and all three
  aliases — `main`, `nightly`, `nightly-7b6e960…` — were verified in the
  registry as multi-arch manifests resolving to the same digest on all
  three services (backend, frontend, push-proxy). A subsequent automatic
  `workflow_run` promotion of the same commit (run 36729587535)
  re-applied the aliases moments later, so the point-in-time digests
  moved while the per-service alias equality held.
- `fix(websocket)`: issue #320 (ghost reconnect after explicit `disconnect()`)
  fixed — close/onerror handlers detached before `ws.close()` so the late
  close event cannot re-arm the reconnect path; the orphaned-CONNECTING-socket
  route closed in the same change; regression-tested (#331).
- `refactor(config)`: second live config store eliminated (#334) — the legacy
  `stores/config.ts` and `features/config` store ran simultaneously with only
  the legacy one wired for live updates; migrated with the legacy auth-merge
  guard ported and a regression test.
- `chore(governance)`: M7(a) executed (#335, ADR-007).
- `docs`: current-state docs corrected against the actual tree (#336) —
  the refactoring notes and architecture diagram still described deleted
  files (legacy `stores/`, `WebSocketManager`, per-feature service/repo
  layers) and nonexistent error-handling conventions; every corrected
  path/count re-measured; historical point-in-time records untouched.
- Follow-ups filed, deliberately not bundled: #332 (moving-alias push race,
  pre-existing, exposed by the promotion fix), #333 (`updateConnectionStatus`
  time-based branches are dead code — behavioral change, needs a decision).

## 5. Tier 3 — large items (design first)

1. **Durable realtime replay/outbox** — the single highest-impact technical gap.
   Needs its own spec (likely an ADR): persist client-facing events with per-connection
   sequence continuity across restarts, bounded retention, replay-on-reconnect, and
   explicit `resync_required` beyond the window. Reuse patterns from the existing
   Buzz durable outbox (`backend/src/integrations/outbox.rs`) where applicable.
   Design note: `docs/plans/2026-09-29-tier3-durable-realtime-replay-design.md`
   (#326) — premise re-verified (three staleness corrections, incl. that window
   overflow is silent today and `resync_required` has no frontend consumer),
   adversarially design-reviewed (two blockers fixed pre-merge), awaiting maintainer
   sponsorship; implementation not started.
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
PR-1 (docs) ─┬─ parallel ─┬─ PR-2 (frontend dead code)     } all merged
             │            ├─ PR-3 (backend unwraps)         } 2026-09-28/29
             │            └─ PR-4 (SVG hardening)           } (#306–#312)
PR-5 (triage #88/#89) ──> M1 ──> M8 (release v0.5.1)        } #310/#312; M1 done
close #258 (no PR)                                            } done (live fix + #311)

Remaining order (updated 2026-09-30): M8 (release v0.5.1, maintainer-gated) →
M2 (awaiting sponsorship) → M4 enforcing flip (maintainer manual pass) →
Tier 3 by spec. M3/M5/M6/M7 delivered (see §4).
```
