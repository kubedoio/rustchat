# Agent Operating Model

**Last updated:** 2026-03-22
**Canonical agent guide:** [`AGENTS.md`](https://github.com/kubedoio/rustchat/blob/main/AGENTS.md) (repository root) — read that first for full workflow details

---

## Overview

This document describes the LLM agent operating model for rustchat: which agents are authorized, what skills they use, and what scope boundaries apply. It structures the information per the LLM Development Operating Model v2.0.

The machine-readable contracts live in `.governance/agent-contracts.yml`. This doc is the human-readable summary.

---

## 1. Agent Roles

Three agents are defined for rustchat:

| Agent | Mode | Role |
|---|---|---|
| `backend-agent` | bounded-write | Implements backend features and fixes |
| `frontend-agent` | bounded-write | Implements frontend features and fixes |
| `compat-agent` | read-only | Performs Mattermost compatibility analysis, produces design artifacts |

### backend-agent

**Allowed paths:** `backend/src/**`, `backend/tests/**`, `backend/migrations/**`, `push-proxy/**`

**Prohibited without explicit approval:**
- `backend/src/auth/**` — requires explicit human approval
- `backend/src/api/v4/**` — requires compat-reviewer co-approval (`@zoorpha`)
- `backend/src/mattermost_compat/**` — requires compat-reviewer co-approval
- `backend/src/a2a/**` — requires senior review
- `.governance/**`, `frontend/**` — always prohibited

**Limits:** ≤10 files, ≤300 lines per PR. Tests required. Human review required.

**ADR required for:** auth changes, permission changes, API contract changes.

### frontend-agent

**Allowed paths:** `frontend/src/**`, `frontend/e2e/**`

**Prohibited:** `backend/**`, `.governance/**`

**Limits:** ≤10 files, ≤300 lines per PR. Human review required.
**Tests:** Not required (explicitly set to `false` in contract).

### compat-agent

**Mode:** Read-only — cannot write production code, cannot approve PRs.

**Can read:** `backend/compat/**`, `backend/src/api/v4/**`, `backend/src/mattermost_compat/**`, `tools/mm-compat/**`

**Can write (analysis output only):** `previous-analyses/**`, `docs/superpowers/specs/**`, `docs/superpowers/plans/**`
**Prohibited:** `.governance/**` — always prohibited.

**Applicable skills:** `mattermost-api-parity`, `mm-endpoint-contract-parity`, `mm-websocket-calls-parity`, `mm-mobile-journey-parity`

---

## 2. Skills System

The following capability skills are used by the maintainer agent workflow. Skill definitions are maintained as internal tooling and are not kept in the public repository tree.

Skills:

| Skill | Domain |
|---|---|
| `mattermost-api-parity` | Compat: Mattermost HTTP API analysis |
| `mm-endpoint-contract-parity` | Compat: endpoint contract comparison |
| `mm-websocket-calls-parity` | Compat: WebSocket event contract analysis |
| `mm-mobile-journey-parity` | Compat: mobile user journey coverage |
| `production-readiness-gate` | Quality: pre-release checklist |
| `user-validation` | Quality: user-facing behavior verification |
| `ai-summarization-rag` | Feature: AI summarization / RAG pipeline |

**Relationship to agent contracts:** Skills define *how* (capability, method, output format). Contracts define *where* (allowed paths, scope boundaries). A skill tells the agent what to do; the contract tells it where it is allowed to do it.

For compat-agent: only the 4 compat skills apply. For backend-agent and frontend-agent: any non-compat skill.

---

## 3. Workflow

All agent work follows the **plan-first** workflow:

```
1. Brainstorm → spec (docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md)
2. Write plan (docs/superpowers/plans/YYYY-MM-DD-<topic>.md)
3. Execute plan task-by-task (subagent-driven-development or executing-plans)
4. Two-stage review per task: spec compliance → code quality
5. finishing-a-development-branch
```

No ad-hoc changes to protected paths. No implementation without a prior spec.

---

## 4. Compatibility-First Rule

Any change to the Mattermost compatibility surface (`backend/src/api/v4/**`, `backend/src/mattermost_compat/**`, `backend/compat/**`) requires:

1. Compat reviewer co-approval (`@zoorpha` per `CODEOWNERS`)
2. Risk tier elevated (automatic per `protected-paths.yml`)
3. Compat contract tests pass

When in doubt whether a change touches the compat surface, treat it as if it does.

---

## 5. Boundaries Summary

| Area | backend-agent | frontend-agent | compat-agent |
|---|---|---|---|
| `backend/src/` (non-protected) | ✅ write | ❌ | ✅ read |
| `backend/src/api/v4/` | ⚠️ co-approval | ❌ | ✅ read |
| `backend/src/auth/` | ⚠️ explicit approval | ❌ | ❌ |
| `backend/src/mattermost_compat/` | ⚠️ co-approval | ❌ | ✅ read |
| `backend/src/a2a/` | ⚠️ senior review | ❌ | ❌ |
| `backend/migrations/` | ✅ write | ❌ | ❌ |
| `backend/tests/` | ✅ write | ❌ | ❌ |
| `backend/compat/` | ❌ | ❌ | ✅ read |
| `frontend/src/` | ❌ | ✅ write | ❌ |
| `frontend/e2e/` | ❌ | ✅ write | ❌ |
| `.governance/` | ❌ | ❌ | ❌ |
| `docs/superpowers/specs/` | ❌ | ❌ | ✅ write |
| `docs/superpowers/plans/` | ❌ | ❌ | ✅ write |

---

## 6. Cross-References

- Full agent workflow guide: [`AGENTS.md`](https://github.com/kubedoio/rustchat/blob/main/AGENTS.md)
- Machine-readable contracts: [`.governance/agent-contracts.yml`](https://github.com/rustchatio/rustchat/blob/main/.governance/agent-contracts.yml)
- Risk tier definitions: [`.governance/risk-tiers.yml`](https://github.com/rustchatio/rustchat/blob/main/.governance/risk-tiers.yml)
- Protected paths: [`.governance/protected-paths.yml`](https://github.com/rustchatio/rustchat/blob/main/.governance/protected-paths.yml)
- Code ownership: [`.github/CODEOWNERS`](https://github.com/rustchatio/rustchat/blob/main/.github/CODEOWNERS)
