# P001 — Final Legacy Release and Buzz Pivot Bootstrap

**Status:** Draft — requires architecture-owner approval before execution

Use the following prompt with a repository-capable coding agent. Execute it in reviewed phases; do not allow the agent to merge, tag, publish, or delete branches without explicit human approval.

---

## Prompt

You are the implementation agent for the RustChat architectural pivot. Work as a cautious staff-level software architect and release engineer. The authoritative documents are:

- `docs/adr/ADR-004-buzz-upstream-foundation.md`
- `docs/pivot/README.md`
- `docs/pivot/00-final-pre-buzz-release.md`
- `docs/pivot/SPEC-001-repository-and-upstream-boundaries.md`
- `docs/pivot/SPEC-002-enterprise-control-plane.md`
- `docs/pivot/CONTRACTS.md`
- `docs/pivot/TEST-STRATEGY.md`
- `docs/pivot/FEASIBILITY.md`
- `docs/pivot/LLM-ALIGNMENT.md`
- `docs/pivot/PROMPT-LEDGER.md`

Never override them silently. When code and documents disagree, stop that slice and report the conflict. The alignment rules in `LLM-ALIGNMENT.md` (A1–A10) are binding on you; the prohibited-behavior list (A7) applies verbatim.

This is a master prompt. Each phase below is executed through its own gated execution prompt (P002 onward) that inherits these constraints; do not attempt all phases in one session.

Integration assumptions must come from `FEASIBILITY.md`, which names source-verified upstream seams (relay membership gate, NIP-OA attestation, NIP-IA archive, audit log, `/query` bridge) and known gaps (no web chat client, no FCM profile, no NIP-46). If you believe a new seam exists, cite the upstream file path that proves it before using it; an uncited assumption is treated as a hallucination.

### Goal

Complete the final standalone RustChat release and establish a new, extremely clean architecture in which:

1. The existing standalone RustChat code is preserved by a final release tag and `legacy/0.5` branch.
2. `block/buzz` is the authoritative upstream collaboration core.
3. `kubedoio/buzz` is a minimal tracked fork used for branding, packaging, configuration hooks, and upstream-pending fixes.
4. `kubedoio/rustchat` owns enterprise services, RustShare integration, contracts, conformance tests, deployment, and distribution metadata.
5. Mattermost compatibility and the Vue frontend are not carried into the new architecture.
6. Buzz protocol, event semantics, auth behavior, and schema ownership remain upstream compatible.

### Absolute prohibitions

Do not:

- copy the Buzz source tree into the RustChat repository
- merge the old RustChat backend with the Buzz relay
- translate old RustChat database tables into permanent Buzz abstractions
- add Mattermost compatibility to Buzz
- implement RustChat business logic inside Buzz core crates
- read or write Buzz database tables from enterprise services
- edit previously published upstream Buzz migrations
- invent downstream-only event semantics when supported Buzz events/contracts can be used
- disable, delete, skip, or weaken upstream tests
- use mutable `latest`, `main`, or unpinned dependencies in a release matrix
- place secrets, tokens, passwords, or private keys in source, prompts, fixtures, or logs
- treat generated code as accepted without tests and review
- create one giant PR containing release, fork, identity, RustShare, mobile, and compliance work

### Phase A — Inspect and report

Before changing code:

1. Inspect repository status, current versions, release workflows, tags, open PRs, and CI.
2. Verify whether tag `v0.5.1` already exists.
3. Identify version mismatches across backend, frontend, push proxy, images, and documentation. The push proxy is retired with the final release; note it in the release notes as end-of-life alongside the backend and Vue client.
4. Identify release blockers only; do not propose new legacy features.
5. Produce a written execution plan with separate PRs and rollback points.
6. Record the planned execution under P001 in `PROMPT-LEDGER.md`.

Required output: inspection report and proposed PR sequence. Do not modify release tags yet.

### Phase B — Final standalone release preparation

Create a dedicated release-preparation PR that:

1. Aligns all standalone component versions to the approved final version.
2. Corrects repository and artifact metadata.
3. Adds accurate final-release notes and known limitations.
4. Fixes only release-blocking failures.
5. Runs the complete current CI and release checks.
6. Adds a reproducible clean-install smoke test if absent.
7. Produces artifact/SBOM/provenance steps supported by the current project.

Required output:

- release-preparation PR
- exact tested commit SHA
- complete test report
- proposed signed tag command or release action
- no tag or release publication without human approval

After approval and merge, create a separate release action for:

- signed tag `v0.5.1`
- immutable artifacts
- release notes
- `legacy/0.5` from the same commit
- branch protection recommendation

### Phase C — Create the downstream Buzz operating model

Do not modify the RustChat legacy code for this phase.

1. Establish `kubedoio/buzz` as a fork of `block/buzz`.
2. Document `origin` and `upstream` remotes.
3. Add a patch ledger with the classes and thresholds from SPEC-001.
4. Add automated upstream synchronization that opens a PR rather than updating production directly.
5. Preserve all upstream CI commands and tests.
6. Add checks that detect edited published migrations, event-kind divergence, protocol-fixture divergence, and unregistered downstream patches.
7. Add data-driven branding configuration where possible.
8. Keep the first fork PR limited to operational discipline and branding infrastructure; do not implement enterprise identity yet.

Required output:

- fork/bootstrap PR
- upstream base SHA
- downstream patch count
- unchanged upstream test report
- generated divergence report

### Phase D — Scaffold the RustChat distribution repository

Transform forward development in `kubedoio/rustchat` using a new branch only after the legacy release is preserved.

Create this structure:

```text
contracts/
services/
deploy/
distribution/
tests/conformance/
tests/e2e/
tests/upgrade/
scripts/pivot/
scripts/release/
```

Requirements:

1. Add a machine-readable pinned distribution version matrix.
2. Reference Buzz by immutable commit/image digest; do not copy source.
3. Add schemas for every contract in `docs/pivot/CONTRACTS.md`.
4. Add contract-test harnesses before service implementations.
5. Add CI architecture guards.
6. Add Compose and Helm skeletons with secrets referenced externally.
7. Add backup, restore, and upgrade test skeletons.
8. Add clear `README` documentation acknowledging Buzz as upstream.
9. Remove or archive legacy build paths only in a dedicated reviewed PR after the legacy tag exists.

Required output: scaffold PR that builds/tests without implementing broad product features.

### Phase E — Implement vertical slices in order

Use separate prompts and PRs for each slice. Do not proceed to the next slice until the previous contract and tests pass.

1. Keycloak Authorization Code + PKCE, identity binding, and key-possession proof.
2. Suspension, revocation, reactivation, and organization membership lifecycle.
3. RustShare MCP retrieval with permission intersection and citations.
4. Organization/group synchronization and policy mapping.
5. Versioned distribution, backup, restore, upgrade, and rollback.
6. Desktop/web/mobile rebranding and enterprise login.
7. Push, deep links, signing, and store release pipelines.
8. Compliance export and retention only after lifecycle stability.

For each slice:

- create/update the corresponding prompt entry
- begin with tests from the contract
- implement the smallest vertical behavior
- run upstream and downstream tests
- document assumptions and security decisions
- provide rollback steps
- do not merge automatically

### Required architecture checks in every PR

Every PR must answer:

1. Does this modify Buzz core? If yes, why can it not be external or upstreamed?
2. Does it change protocol, event semantics, auth, schema, or data ownership?
3. Is every downstream patch in the patch ledger?
4. Is there an upstream issue or PR when the change is generally useful?
5. Are all dependencies pinned for a release?
6. Are contracts and producer/consumer tests updated?
7. Do suspension and authorization paths fail closed?
8. Are private data and secrets excluded from logs?
9. Did all unchanged upstream tests remain enabled?
10. Is rollback documented?

### Testing requirements

Follow `docs/pivot/TEST-STRATEGY.md`. At minimum:

- run unmodified upstream Buzz tests
- run fork-divergence checks
- run contract tests
- run identity and authorization intersection tests
- run RustShare permission/evidence tests
- run desktop/web/mobile smoke tests for affected clients
- run clean install, backup/restore, and upgrade tests for distribution changes

Never change tests only to match a failing generated implementation. Resolve the implementation or document a reviewed contract change.

### Commit and PR discipline

- Use small, intentional commits.
- One architecture slice per PR.
- Draft PRs by default.
- Include prompt ID in the PR description.
- Include upstream base and downstream head SHAs.
- Include exact commands and results for tests.
- Include architecture exceptions explicitly; `none` is a valid value.
- Never merge or publish without human review.

### Final execution report

For each completed phase, update `PROMPT-LEDGER.md` with:

- repository and branch
- input and output SHAs
- PR link
- assumptions
- files changed
- tests run and omitted
- downstream patch changes
- upstream issue/PR links
- exceptions
- result and next prompt ID

If any requested action conflicts with the architecture rules, refuse that action in the execution report and propose the nearest compliant alternative.

---

## Expected PR sequence

1. Governance and pivot-plan PR.
2. Final standalone release-preparation PR.
3. Final release/tag action and legacy branch.
4. Kubedo Buzz fork/bootstrap PR.
5. RustChat distribution scaffold PR.
6. Identity-binding vertical slice.
7. Lifecycle/revocation vertical slice.
8. RustShare retrieval vertical slice.
9. Distribution/operations vertical slice.
10. Client rebranding and mobile release slices.

The sequence is deliberately incremental. Do not combine it for speed.
