# Ownership Map

**Last updated:** 2026-09-26  
**Sources of truth:** [`.github/CODEOWNERS`](https://github.com/kubedoio/rustchat/blob/main/.github/CODEOWNERS),
[`.governance/risk-tiers.yml`](https://github.com/kubedoio/rustchat/blob/main/.governance/risk-tiers.yml), and
[`.governance/protected-paths.yml`](https://github.com/kubedoio/rustchat/blob/main/.governance/protected-paths.yml)

---

## Overview

CODEOWNERS determines review routing; risk tiers determine the required review
and evidence level.

GitHub CODEOWNERS uses **last matching pattern wins** semantics. Broad ownership
rules must therefore appear before their specific overrides.

All unmatched paths are owned by `@senolcolak`.

A CODEOWNERS entry containing multiple users does **not** by itself mean every
listed user must approve. The repository review policy is defined by
`.governance/risk-tiers.yml`:

- every merged PR requires at least one independent human approval;
- elevated and architectural changes also require maintainer
  sponsorship/sign-off;
- the PR author does not count as the independent reviewer; a maintainer author may satisfy maintainer sponsorship but still needs a separate independent approval;
- for the highest-risk architectural changes, seek a second independent review
  when the active reviewer pool permits.

---

## 1. Area Map

| Path | Human owner(s) | Agent authorized | Minimum risk tier |
|---|---|---|---|
| `backend/src/` (non-protected) | `@senolcolak` | `backend-agent` | standard |
| `backend/src/auth/**` | `@senolcolak` | `backend-agent` ⚠️ explicit approval | elevated |
| `backend/src/api/v4/**` | `@senolcolak` + `@zoorpha` | `backend-agent` ⚠️ compat review | elevated |
| `backend/src/mattermost_compat/**` | `@senolcolak` + `@zoorpha` | `backend-agent` ⚠️ compat review | elevated |
| `backend/src/realtime/**` | `@senolcolak` | `backend-agent` | elevated |
| `backend/src/integrations/**` | `@senolcolak` | bounded backend work only; architectural review required | architectural |
| `backend/migrations/**` | `@senolcolak` | `backend-agent` | elevated |
| `backend/tests/**` | `@senolcolak` | `backend-agent` | standard |
| `backend/compat/**` | `@senolcolak` + `@zoorpha` | `compat-agent` (read) | elevated |
| `push-proxy/**` | `@senolcolak` | `backend-agent` | standard |
| `frontend/**` | `@senolcolak` | `frontend-agent` | standard |
| `tools/mm-compat/**` | `@senolcolak` + `@zoorpha` | `compat-agent` (read) | elevated |
| `.github/**` | `@senolcolak` | none | standard unless protected below |
| `.github/CODEOWNERS` | `@senolcolak` + `@zoorpha` | none | elevated |
| `.github/pull_request_template.md` | `@senolcolak` | none | elevated |
| `.governance/**` | `@senolcolak` + `@zoorpha` | none | architectural |
| `GOVERNANCE.md` | `@senolcolak` + `@zoorpha` | none | architectural |
| `docs/**` | `@senolcolak` | `compat-agent` only in its configured analysis-output paths | standard unless protected below |
| `docs/adr/**` | `@senolcolak` + `@zoorpha` | none | architectural |
| `docs/github-protection.md` | `@senolcolak` | none | elevated |
| `docker/**` | `@senolcolak` | none | standard |
| `scripts/**` | `@senolcolak` | none | standard |
| `*` (everything else) | `@senolcolak` | — | standard |

---

## 2. Co-Owned / Specialist Review Areas

These paths route review to more than one domain owner. Review requirements come
from the risk tier, not from counting CODEOWNERS entries.

| Area | Owners | Why |
|---|---|---|
| `backend/compat/**` | `@senolcolak` + `@zoorpha` | Compatibility contracts |
| `backend/src/api/v4/**` | `@senolcolak` + `@zoorpha` | Mattermost HTTP compatibility |
| `backend/src/mattermost_compat/**` | `@senolcolak` + `@zoorpha` | Compatibility utilities |
| `tools/mm-compat/**` | `@senolcolak` + `@zoorpha` | Compatibility analysis tooling |
| `.github/CODEOWNERS` | `@senolcolak` + `@zoorpha` | Ownership changes affect review routing |
| `.governance/**` | `@senolcolak` + `@zoorpha` | Repository-wide policy |
| `GOVERNANCE.md` | `@senolcolak` + `@zoorpha` | Repository-wide governance |
| `docs/adr/**` | `@senolcolak` + `@zoorpha` | Architectural decisions |

For architectural work, one independent approval plus maintainer sponsorship /
sign-off is mandatory under the current reviewer-pool policy. Authentication,
storage/data-model, protocol, and governance changes should seek a second
independent review when available.

---

## 3. Risk Tier Auto-Elevation

Changes to any path in `.governance/protected-paths.yml` are automatically
elevated to the minimum tier listed there.

A PR touching `backend/src/api/v4/**`, for example, is always at least
`elevated` regardless of how small the diff is. A PR touching `docs/adr/**` or
`.governance/**` is architectural.

See `.governance/protected-paths.yml` for the complete list.

---

## 4. Agent Boundaries

| Agent | Authorized areas | Prohibited / gated |
|---|---|---|
| `backend-agent` | `backend/src/`, `backend/tests/`, `backend/migrations/`, `push-proxy/` | `frontend/`, `.governance/`, and protected sub-paths without required approval |
| `frontend-agent` | `frontend/src/`, `frontend/e2e/` | `backend/`, `.governance/` |
| `compat-agent` | Read compat surface; write only configured analysis-output paths | All production code paths |

For exact machine-readable boundaries see
[`.governance/agent-contracts.yml`](https://github.com/kubedoio/rustchat/blob/main/.governance/agent-contracts.yml).
For the operating model see
[`docs/development/operating-model.md`](./operating-model.md) and
[`AGENTS.md`](https://github.com/kubedoio/rustchat/blob/main/AGENTS.md).

---

## 5. How to Update Ownership

Changing CODEOWNERS is an `elevated` change unless the same PR also changes
repository-wide governance/ADR policy, in which case the higher architectural
tier applies.

Required process:

1. Edit `.github/CODEOWNERS`.
2. Keep broad patterns before specific overrides because CODEOWNERS is
   last-match-wins.
3. Obtain the independent review and maintainer sign-off required by the
   applicable risk tier.
4. Verify live branch protection still requires CODEOWNERS review where intended.
