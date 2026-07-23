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

The following prompts should be created only when the prior gate is approved:

- `P002` — Execute and verify final `v0.5.1` standalone release.
- `P003` — Create `kubedoio/buzz` fork discipline, patch ledger, and upstream-sync workflow.
- `P004` — Implement data-driven RustChat branding across Buzz clients.
- `P005` — Implement Keycloak identity binding and revocation contract.
- `P006` — Implement RustShare MCP and permission-intersection contract.
- `P007` — Build version-matrix, Compose, Helm, backup, and restore distribution.
- `P008` — Add mobile signing, push, deep links, and store-release gates.
- `P009` — Implement compliance export and retention after lifecycle stability.

Do not generate all implementation PRs in one operation. Each prompt must produce a reviewable vertical slice with independent rollback.
