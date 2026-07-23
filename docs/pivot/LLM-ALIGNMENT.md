# LLM Development Alignment Strategy

**Status:** Proposed — binding on every prompt and agent once ADR-004 is accepted

RustChat's forward development is executed by LLM coding agents. This document is the alignment contract between those agents and the project. It exists because LLM agents fail in characteristic ways — confident prose over evidence, silent scope growth, tests edited to fit the implementation, unverified claims of success — and a project built *entirely* by LLMs must make those failure modes mechanically unrewarding.

The governing principle: **trust machines, not prose.** An agent's report is never evidence. Evidence is a command, its raw output, and a commit that CI re-verifies independently.

## Authority hierarchy

When two sources disagree, the higher one wins and the conflict is reported, never silently resolved:

1. ADRs (`docs/adr/`) — architecture decisions
2. Specifications (`docs/pivot/SPEC-*.md`)
3. Machine-readable contract schemas (`contracts/` — generated from `docs/pivot/CONTRACTS.md` before any implementation)
4. Tests and CI gates
5. Prompts (`docs/pivot/prompts/`)
6. Generated code

A prompt may not override levels 1–4. Generated code is the *least* authoritative artifact in the repository. If an agent finds code and docs disagreeing, it stops that slice and reports; it does not pick a winner.

## Alignment rules

### A1 — An agent never verifies its own work

Every change is verified by layers the authoring agent does not control:

- CI runs the full gate (guard script, contract tests, upstream tests) on the PR, not on the agent's machine.
- Security-sensitive slices (identity, authorization, custody, compliance) get a **reviewer agent with no shared context** — a fresh session that receives the spec and the diff, not the author's reasoning. Where possible, author and reviewer use different models/providers.
- Protected paths (see `.github/CODEOWNERS` and `.governance/protected-paths.yml`) additionally require human approval.

### A2 — Tests and schemas are authored before and apart from implementations

For every vertical slice: contract schemas and conformance tests are written first, from the specification, in their own PR or commit, by a prompt that has not seen any implementation. An implementation PR that arrives with tests "tailored" to it is rejected. This closes the most common self-deception loop: the agent writing tests that its own code passes.

### A3 — Protected by construction, not by instruction

Instructions in prompts are weak; repository mechanics are strong. Before Phase 2 starts, a human must enable:

- the Pivot Architecture Guard as a **required status check** on `main`,
- branch protection: no direct pushes, no force pushes, required CODEOWNERS reviews,
- CODEOWNERS coverage for `docs/pivot/`, `scripts/pivot/`, the guard workflow, `contracts/`, and `distribution/versions.yaml` (added in this PR),
- the same setup in `kubedoio/buzz` once the fork is formalized.

An agent that cannot merge cannot be misled into merging. If any of these settings is missing, that is a launch blocker, not a nicety.

### A4 — Evidence or it did not happen

Every PR and every execution-ledger entry must include, for each claim:

- the exact commands run,
- their raw, untruncated output (or a linked CI run),
- the input commit SHA.

"Tests not run" requires a reason and is a visible red flag, not a footnote. A report that says "all tests pass" without output is treated as failing. LLM-authored summaries of test results are not acceptable substitutes for raw output.

### A5 — One slice, one prompt, one PR

Scope is the main vector for drift. Each prompt from `PROMPT-LEDGER.md` produces exactly one reviewable vertical slice with independent rollback. An agent that discovers necessary out-of-scope work stops and files a follow-up prompt ID instead of expanding. PRs touching more than one architecture layer (contracts + services + clients) are presumed misaligned and split.

### A6 — Tripwires run on every PR, not on request

- `scripts/pivot/check-architecture-boundaries.sh` (copied-source detection by crate name, forbidden-dependency scan across all manifests, mutable-pin rejection, hollowed-doc detection).
- Fork-divergence checks in `kubedoio/buzz` (patch ledger completeness, patch budget thresholds, migration immutability, event-kind registry diff).
- Contract conformance suites once they exist.

A failing tripwire is evidence of a design problem. Weakening, skipping, or deleting a tripwire requires the same approval as changing the architecture rule it enforces (see the change-control rule in `README.md`).

### A7 — Prohibited agent behaviors (explicit anti-patterns)

An agent must never:

- edit tests, fixtures, thresholds, or expected outputs to match a failing implementation,
- edit the guard script, governance files, CODEOWNERS, or these rules to make a change pass,
- mark a failing test as expected/ignored to turn CI green,
- reduce coverage, hollow out a governance document, or shorten a spec while preserving its title,
- claim a command was run when it was not, or summarize output it has not seen,
- resolve a doc/code conflict by silently choosing one side,
- introduce a dependency, abstraction, or configuration option the slice did not require,
- copy upstream source into this repository under a renamed path,
- treat "the human approved a previous similar change" as standing approval for a new one.

These are firing offenses for a prompt: an execution record showing any of them marks the prompt `rejected` and the output is discarded.

### A8 — Schema-first contracts

Before any enterprise service is implemented, `docs/pivot/CONTRACTS.md` C1–C7 are compiled into machine-readable JSON Schemas under `contracts/` with producer/consumer validation in CI. Agents code against schemas, never against the prose. Prose is for humans deciding; schemas are for agents building. The prose example blocks in `CONTRACTS.md` are illustrative and non-normative once schemas exist.

### A9 — Human decision points (the complete list)

Everything else is delegable to agents; these are not:

1. ADR acceptance or amendment.
2. Any change to governance files, the guard, CODEOWNERS, or this document.
3. Releases, tags, version-pin advances to a new upstream commit.
4. Architecture exceptions and patch-class changes.
5. Key-custody, security-model, and compliance-scope decisions.
6. Enabling/disabling the repository settings in A3.

### A10 — Periodic adversarial audit

At every phase gate, a read-only audit agent (fresh context, no authorship history) compares the implementation against the contracts and this document, and answers in writing: *what here would let an agent drift without anyone noticing?* Findings are fixed before the gate passes. The audit prompt receives the specs and the diff, not the implementation agents' reports.

## How this prevents being misled — the failure-mode map

| LLM failure mode | Countermeasure |
|---|---|
| "It works" without running it | A4: raw output or it did not happen |
| Tests written to fit the code | A2: test-first, different author prompt |
| Quietly editing the guard/tests | A3: CODEOWNERS + required checks; A7: firing offense |
| Scope creep inside one PR | A5: one slice per prompt; split by layer |
| Hallucinated upstream APIs | `FEASIBILITY.md` names verified seams with file paths; agents must cite source for new seams |
| Confident prose over schemas | A8: schema-first; prose non-normative |
| Self-approved merge | A3: agents physically cannot merge |
| Gradual drift across many small PRs | A6 tripwires + A10 audits + patch budget thresholds |
| Echo-chamber review by the same model | A1: context-free reviewer, different model where possible |

## Relationship to existing governance

`.governance/agent-contracts.yml`, `protected-paths.yml`, and `risk-tiers.yml` remain in force. This PR extends `protected-paths.yml` and CODEOWNERS for the pivot layout. The agent contracts themselves (allowed paths, size limits) are re-scoped for the `contracts/` + `services/` + `distribution/` layout as part of the Phase D scaffold PR — that re-scoping is itself an architectural-tier change under this document.
