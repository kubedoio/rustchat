# RustChat Pivot Prompt Ledger

This ledger tracks prompts used to implement the Buzz-based RustChat architecture. It is part of the engineering record, not a disposable chat history.

## Rules

- Every implementation prompt receives a stable ID: `P###`.
- Prompts are committed before or with the code they direct.
- A prompt may not override an ADR, specification, contract, or test gate.
- The resulting PR records the prompt ID, model/tool used, assumptions, files changed, tests run, and deviations.
- Follow-up prompts use a new ID when they materially change scope or architecture.
- Secrets, tokens, customer data, and private keys must never be stored in prompts.
- A successful prompt is not evidence that the implementation is correct; contracts and tests are authoritative.
- Every claimed verification in an execution record includes the exact command and raw output or a linked CI run (LLM-ALIGNMENT rule A4). A record without evidence is treated as failing.
- Master prompts and execution prompts have distinct roles: a master prompt (P001) defines phases and prohibitions; each phase is executed only through its own gated execution prompt (P002, P003, …). A master prompt is never executed end-to-end in one session.

## Status values

- `draft`
- `approved`
- `executing`
- `completed`
- `superseded`
- `rejected`

## Registry

| ID | Title | Status | Scope | Output expectation |
|---|---|---|---|---|
| [P001](./prompts/P001-final-release-and-buzz-pivot-bootstrap.md) | Final legacy release and Buzz pivot bootstrap | draft | Freeze standalone RustChat and establish the clean Buzz-based structure | Release PRs, fork/bootstrap PRs, contracts, CI guards, and execution report |

## Prompt creation records

### P001 authored 2026-07-23

- Prompt: `docs/pivot/prompts/P001-final-release-and-buzz-pivot-bootstrap.md`
- Model/tool: GPT-5.6 Thinking with GitHub connector
- Operator: Senol Colak / ChatGPT
- Repository and branch: `kubedoio/rustchat`, `agent/freeze-legacy-and-plan-buzz-pivot`
- Input commit: `73b7e566b2c4c31db0a4e7aa0e570f43de684ee0`
- Governance PR: https://github.com/kubedoio/rustchat/pull/230
- Scope created: legacy-release guide, ADR-004, repository boundaries, enterprise-control-plane spec, contracts, test strategy, prompt ledger, master prompt, and CI architecture guard
- Runtime product code changed: none
- Validation: branch compared with `main`; GitHub CI requested through the draft PR
- Local validation limitation: outbound DNS prevented a fresh clone in the authoring environment
- Architecture exceptions: none
- Prompt execution status: not started; P001 remains draft until PR #230 is reviewed and ADR-004 is accepted

### P001 review amendment 2026-07-23

- Amendment author: Kimi Code CLI (Moonshot AI), interactive review session
- Operator: Senol Colak
- Trigger: architecture review of PR #230 (alignment, feasibility, and enforcement gaps)
- Changes: added `FEASIBILITY.md` (source-verified spike against `block/buzz` @ `6a56c8b`, including the web-client, FCM, and NIP-46 gaps); added `LLM-ALIGNMENT.md` (rules A1–A10); hardened `scripts/pivot/check-architecture-boundaries.sh` (crate-name copied-source detection, repo-wide dependency scan, scoped submodule URL check, hollowed-doc detection, extended mutable-ref list); extended `.github/CODEOWNERS` and `.governance/protected-paths.yml` for the pivot layout; amended ADR-004 (explicit Nostr commitment, bounded bridge loophole, velocity/feasibility risks), README (push-proxy retirement, A3 enforcement gate, upstream-PR backlog, Phase 4 web decision), CONTRACTS (NIP-OA as the C3 mechanism, concrete C2 SLA), SPEC-001 (enforcement section, honest residual limits), SPEC-002 (user-owned custody for first preview, verified seams), TEST-STRATEGY (LLM policy alignment), and P001 (alignment references, verified-seam guidance)
- Local validation: `bash scripts/pivot/check-architecture-boundaries.sh` executed locally; passes on the amended tree
- Architecture exceptions: none

## Execution record template

Copy this section when a prompt is executed:

```markdown
### P### execution YYYY-MM-DD

- Prompt: `docs/pivot/prompts/P###-...md`
- Model/tool:
- Operator:
- Repository and branch:
- Input commit:
- Output commit/PR:
- Assumptions made:
- Files changed:
- Tests run:
- Tests not run and why:
- Architecture exceptions:
- Contract changes:
- Upstream changes/PRs:
- Result: completed | partial | rejected
- Follow-up prompt IDs:
```

## Planned prompt sequence

The following prompts should be created only when the prior gate is approved. P001 is the master prompt: it defines the phases and prohibitions, but each phase below is executed through its own gated prompt that inherits P001's constraints — P001 itself is never run end-to-end in one session.

- `P002` — Execute and verify final `v0.5.1` standalone release (P001 phases A–B).
- `P003` — Create `kubedoio/buzz` fork discipline, patch ledger, upstream-sync workflow, and A3 enforcement settings (P001 phase C).
- `P004` — Implement data-driven RustChat branding across Buzz clients (P001 phase C step 7 / phase E slice 6).
- `P005` — Implement Keycloak identity binding and revocation contract (P001 phase E slices 1–2).
- `P006` — Implement RustShare MCP and permission-intersection contract (P001 phase E slice 3).
- `P007` — Build version-matrix, Compose, Helm, backup, and restore distribution (P001 phase D and phase E slice 5).
- `P008` — Add mobile signing, push via `buzz-push-gateway`, deep links, and store-release gates (P001 phase E slice 7; blocked on the upstream `AppProfile` and FCM profiles per `FEASIBILITY.md`).
- `P009` — Implement compliance export and retention after lifecycle stability (P001 phase E slice 8).

Do not generate all implementation PRs in one operation. Each prompt must produce a reviewable vertical slice with independent rollback.
