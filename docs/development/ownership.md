# Ownership Map

**Last updated:** 2026-03-22
**Source of truth:** [`.github/CODEOWNERS`](https://github.com/rustchatio/rustchat/blob/main/.github/CODEOWNERS)

---

## Overview

Ownership determines who must review a PR before it can be merged. GitHub enforces this automatically when branch protection is enabled.

All paths not listed below are owned by `@senolcolak` (catchall).

---

## 1. Area Map

| Path | Human owner(s) | Agent authorized | Risk tier |
|---|---|---|---|
| `backend/src/` (non-protected) | `@senolcolak` | `backend-agent` | standard |
| `backend/src/auth/**` | `@senolcolak` | `backend-agent` ⚠️ explicit approval | elevated |
| `backend/src/api/v4/**` | `@senolcolak` + `@zoorpha` | `backend-agent` ⚠️ co-approval | elevated |
| `backend/src/mattermost_compat/**` | `@senolcolak` + `@zoorpha` | `backend-agent` ⚠️ co-approval | elevated |
| `backend/src/a2a/**` | `@senolcolak` | `backend-agent` ⚠️ senior review | elevated |
| `backend/src/realtime/**` | `@senolcolak` | `backend-agent` | elevated |
| `backend/migrations/**` | `@senolcolak` | `backend-agent` | elevated |
| `backend/tests/**` | `@senolcolak` | `backend-agent` | standard |
| `backend/compat/**` | `@senolcolak` + `@zoorpha` | `compat-agent` (read) | elevated |
| `push-proxy/**` | `@senolcolak` | `backend-agent` | standard |
| `frontend/**` | `@senolcolak` | `frontend-agent` | standard |
| `tools/mm-compat/**` | `@senolcolak` + `@zoorpha` | `compat-agent` (read) | elevated |
| `.github/**` | `@senolcolak` | none | standard |
| `.github/CODEOWNERS` | `@senolcolak` + `@zoorpha` | none | elevated |
| `.governance/**` | `@senolcolak` + `@zoorpha` | none | architectural |
| `docker/**` | `@senolcolak` | none | standard |
| `scripts/**` | `@senolcolak` | none | standard |
| `docs/**` | `@senolcolak` | `compat-agent` (specs/plans only) | standard |
| `docs/internal/AGENTS.md` | `@senolcolak` | none | standard |
| `*` (everything else) | `@senolcolak` | — | standard |

---

## 2. Dual-Owner Areas

The following areas require **both** owners to approve:

| Area | Owners | Why |
|---|---|---|
| `backend/compat/**` | `@senolcolak` + `@zoorpha` | Compat contracts must have compat-reviewer sign-off |
| `backend/src/api/v4/**` | `@senolcolak` + `@zoorpha` | Mattermost HTTP compat surface |
| `backend/src/mattermost_compat/**` | `@senolcolak` + `@zoorpha` | Compat utilities |
| `tools/mm-compat/**` | `@senolcolak` + `@zoorpha` | Compat analysis tooling |
| `.github/CODEOWNERS` | `@senolcolak` + `@zoorpha` | Ownership changes affect review routing |
| `.governance/**` | `@senolcolak` + `@zoorpha` | Architectural tier — 2 reviewers required |

---

## 3. Risk Tier Auto-Elevation

Changes to any path in `.governance/protected-paths.yml` are automatically elevated to the minimum tier listed. A PR touching `backend/src/api/v4/` is always `elevated` regardless of how trivial the change appears.

See `.governance/protected-paths.yml` for the full list.

---

## 4. Agent Boundaries

| Agent | Authorized areas | Prohibited |
|---|---|---|
| `backend-agent` | `backend/src/`, `backend/tests/`, `backend/migrations/`, `push-proxy/` | `frontend/`, `.governance/`, and protected sub-paths without approval |
| `frontend-agent` | `frontend/src/`, `frontend/e2e/` | `backend/`, `.governance/` |
| `compat-agent` | Read: compat surface. Write: `docs/superpowers/`, `previous-analyses/` | All production code paths |

For full contract details see `.governance/agent-contracts.yml` and `docs/agent-operating-model.md`.

---

## 5. How to Update Ownership

Changing CODEOWNERS is an `elevated` risk tier change (automatic per `protected-paths.yml`). It requires:

1. Edit `.github/CODEOWNERS`
2. Both `@senolcolak` and `@zoorpha` must approve the PR
3. Verify branch protection is still correct after the change

CODEOWNERS uses **last-match wins** semantics — the catchall `*` rule is placed first so specific rules below it take precedence.
