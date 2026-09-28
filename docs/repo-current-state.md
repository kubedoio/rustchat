# Repo Current State

**Last updated:** 2026-09-27  
**Version:** v0.5.1 (source)

> This document describes the state of the repository as of its last update. For live issue tracking see GitHub Issues.

---

## 1. Version

**Current source version:** v0.5.1

> **Note:** The latest *published* GitHub release is currently `v0.4.1`
> (2026-05-22). A `v0.5.0` tag exists in the repository (with `v0.5.0-rc.*`
> prerelease tags) but no `v0.5.0` GitHub Release has been published. The
> `0.5.1` version in source has not been tagged or released yet; publishing
> it is tracked in the release issue. Do not treat `0.5.1` (or `0.5.0`) as a
> published release until a `v0.5.x` tag and GitHub release exist.

Version is synchronized across three files:
- `backend/Cargo.toml` → `[package] version`
- `frontend/package.json` → `"version"`
- `push-proxy/Cargo.toml` → `[package] version`

To check: `grep '^version' backend/Cargo.toml` or `jq .version frontend/package.json`

Versioning follows semver. Releases are cut by pushing a `v*` tag — the
`release.yml` workflow generates the GitHub Release after its configured
validation succeeds.

---

## 2. Services

| Service | Path | Port (default) | Status |
|---|---|---|---|
| Backend API | `backend/` | 3000 | ✅ Active |
| Frontend SPA | `frontend/` | 5173 (dev) / 8080 (via Nginx) | ✅ Active |
| Push Proxy | `push-proxy/` | 3000 (default; `RUSTCHAT_PUSH_PORT`) | ✅ Active |
| Nginx proxy | `frontend/nginx.conf` | 8080 | ✅ Active (Docker) |

**External services required:**
- PostgreSQL 16+ (port 5432)
- Redis 7+ (port 6379)
- S3-compatible storage (port 9000 for RustFS)

For local setup see `docs/running_environment.md`.

---

## 3. Current Architecture

Recent repository work clarified rather than replaced the core architecture.

- Runtime composition is owned by `backend/src/bootstrap/**`.
- Router construction is separated from long-lived worker/dependency startup.
- RustChat remains an independently developed backend/frontend/data model.
- Buzz is an optional external integration behind `backend/src/integrations/buzz/**`.
- The phase-1 Buzz bridge is outbound and uses RustChat-owned mapping/outbox
  state; Buzz availability is not part of the core post transaction.

See:

- `docs/architecture/overview.md`
- `docs/adr/ADR-005-rustchat-core-and-buzz-integration.md`
- `docs/adr/ADR-006-repository-integrity-and-maintainability.md`

---

## 4. Compatibility Status

- **Mobile-critical endpoints:** 41/41 previously tracked endpoints implemented
- **Last compatibility audit recorded:** 2026-05-22
- **Notes:** `POST /api/v4/emoji` and `POST /api/v4/posts/search` are implemented. Post search is currently a pragmatic `ILIKE` implementation, not full Mattermost advanced search parity.

For details see `docs/compatibility-scope.md`.

---

## 5. Known Gaps

| Gap | Area | Priority |
|---|---|---|
| ~~Required/authoritative CI and live repository protections need convergence and evidence~~ — resolved (RI-C01/C02) | governance/release | P0 |
| ~~Promoted artifact publication must be tied to complete release/security gates~~ — resolved (RI-C03 promotion gate) | release | P0 |
| Compliance export is not implemented and is now explicitly labeled as such in the admin UI (no phantom export button); audit-dashboard JSON export fixed. Remaining audit-dashboard defect: summary/failure widgets read membership-policy sync runs (`auto_membership_policy_audit`) while the log list reads the system audit log — see #89 | admin | #89 for the dashboard conflation; compliance export feature is Phase 2 |
| Realtime replay durability and graceful WebSocket shutdown | realtime | Hardening |
| Direct SQL persistence in API handlers: baselined and guarded (RI-C04); bounded migration ongoing | backend architecture | Incremental hardening |
| Large backend modules: baselined and guarded (RI-C05); targeted decomposition ongoing | maintainability | Incremental hardening |
| Release upgrade evidence does not yet cover the full supported migration/recovery matrix (P006) | database/release | Hardening |
| Advanced post search semantics beyond simple `ILIKE` | compat/search | Phase 2 |
| Plugin upload/install/enable/disable/remove flows are compatibility stubs | compat/plugins | Phase 2 |
| LDAP and SAML v4 endpoints are compatibility stubs | compat/enterprise | Phase 2 |

The repository-integrity work is specified in
`docs/plans/2026-09-26-repository-integrity-hardening-spec.md`.

---

## 6. Recently Completed Work

| Phase | Description | Date |
|---|---|---|
| Repository integrity P001 | Authoritative green-main/security aggregates, promotion-gated artifacts, verified protection, dependency fixes | 2026-09 |
| Repository integrity P002 | Live GitHub protection verified + documented (RI-C02) | 2026-09 |
| Repository integrity P003 | API persistence-boundary baseline + guard (RI-C04); site pilot extraction | 2026-09 |
| Repository integrity P004 | Module-growth baseline + guard (RI-C05); posts-service decomposition | 2026-09 |
| Repository integrity P005 | Docs/repository hygiene: release-doc collapse, archive moves, roadmap/current-state update, hygiene CI checks | 2026-09 |
| Runtime/bootstrap cleanup | Single AppState, explicit dependency/runtime composition, supervised long-lived workers, pure router assembly | 2026-09 |
| Buzz integration phase 1 | Optional outbound external bridge with durable outbox, retries, dead-letter handling, encrypted secrets, loop prevention, failure isolation | 2026-09 |
| Documentation direction cleanup | Retired abandoned Buzz pivot program; restored RustChat continuation and ADR-005 | 2026-09 |
| P0 Production Readiness | Security/operations hardening alignment, runbook fixes, and doc/env alignment | 2026-06 |
| AI Agents & Ecosystem | Agent runtime, RAG pgvector capabilities, Tavily web tools, feedback APIs, bot guide, and v0.5.1 alignment | 2026-06 |
| Governance Layer | `.governance/` policy files, CODEOWNERS, PR template, issue forms, branch protection documentation, GitHub labels | 2026-03 |
| Foundation Docs | Structured audience-oriented documentation hierarchy | 2026-03 |
| Entity Foundation | Entity registration, API keys, rate limiting, WebSocket JWT expiry, mobile compatibility audit | 2026-03 |

---

## 7. Quick Start

```bash
# Clone
git clone https://github.com/kubedoio/rustchat
cd rustchat

# Start infrastructure
docker compose up -d postgres redis rustfs

# Backend
cd backend
cargo run

# Frontend (separate terminal)
cd frontend
npm install
npm run dev
```

For full environment setup (env vars, config) see `docs/running_environment.md`.
