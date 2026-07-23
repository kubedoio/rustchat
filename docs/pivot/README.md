# RustChat Buzz Pivot Program

**Status:** Proposed — accepted when the pivot ADR is merged

This directory is the authoritative program guide for ending the standalone RustChat implementation and rebuilding RustChat as a clean enterprise and sovereign distribution on top of upstream Buzz.

## Decision summary

RustChat will:

1. Publish one final pre-Buzz release from the existing codebase.
2. Tag and preserve that code as the immutable legacy line.
3. Stop developing the standalone chat backend, Vue client, Mattermost compatibility layer, and the standalone push proxy. Forward mobile push uses the upstream `buzz-push-gateway` (APNs today; Android/FCM requires an upstream transport profile — see [`FEASIBILITY.md`](./FEASIBILITY.md)).
4. Adopt `block/buzz` as the upstream collaboration core, including its Nostr event and public-key identity model.
5. Maintain a minimal Kubedo Buzz fork for branding, packaging, configuration hooks, and upstreamable fixes only.
6. Build enterprise identity, lifecycle, policy, compliance, deployment, and RustShare integration outside Buzz core.
7. Use the Buzz desktop and Flutter clients as the RustChat clients after rebranding and conformance testing.

## Non-negotiable architecture rules

- Buzz is upstream, not copied source.
- RustChat-specific business logic must not be implemented inside Buzz core.
- No downstream change may alter Buzz event semantics, relay protocol, database schema ownership, or public compatibility without an accepted ADR.
- The Kubedo Buzz fork must remain rebaseable onto `block/buzz/main`.
- Every downstream patch must be listed in a patch ledger with reason, owner, upstream issue/PR, and removal condition.
- Enterprise services communicate with Buzz through documented contracts, events, MCP, CLI, or stable APIs.
- Mattermost compatibility is retired and must not influence the new architecture.
- The legacy code is security-maintenance-only after the final release.
- LLM-generated changes require the same tests, review, and architectural checks as human-written changes, applied per [`LLM-ALIGNMENT.md`](./LLM-ALIGNMENT.md).
- The enforcement layer is real before it is relied on: the architecture guard is a required status check, CODEOWNERS covers the governance and enforcement paths, and agents cannot merge (LLM-ALIGNMENT rule A3). Missing repository settings are launch blockers.
- Feasibility claims are verified against upstream source and re-verified at every phase gate ([`FEASIBILITY.md`](./FEASIBILITY.md)); a failed "feasible" claim reopens ADR-004.

## Target repository model

```text
kubedoio/rustchat
  Enterprise distribution, contracts, deployment, release orchestration,
  identity/control-plane services, RustShare integration, and documentation.

kubedoio/buzz
  Minimal downstream fork of block/buzz. Branding and narrowly scoped hooks only.
  Regularly synchronized with upstream.

block/buzz
  Authoritative upstream collaboration core and clients.
```

The RustChat repository must not vendor the Buzz source tree. During the bootstrap phase it may pin the Kubedo fork through release metadata, container digests, Cargo dependencies, or a Git submodule. A normal copied directory is forbidden.

## Program phases

### Phase 0 — Approve the decision

- Review and merge ADR-004.
- Confirm the final legacy version and release date.
- Confirm that Mattermost compatibility and the Vue frontend are retired.
- Confirm that the project will publicly acknowledge Buzz as upstream.

Exit gate: the ADR is accepted and the final-release checklist has an owner.

### Phase 1 — Final legacy release

Follow [`00-final-pre-buzz-release.md`](./00-final-pre-buzz-release.md).

Exit gate: signed release tag, immutable release artifacts, release notes, archived documentation, and a `legacy/0.5` maintenance branch.

### Phase 2 — Establish upstream discipline

- Fork `block/buzz` into `kubedoio/buzz`. (Note: the fork was created on 2026-07-23 for evaluation, ahead of ADR-004 approval; this phase formalizes it with the discipline below. No downstream patches exist before this phase.)
- Add `upstream` and `origin` remotes in contributor instructions.
- Create an automated upstream-sync workflow.
- Add the downstream patch ledger.
- Create branding configuration that does not change protocol behavior.
- Run upstream unit, integration, desktop, web, and mobile tests unchanged.
- Enable the repository enforcement settings from [`LLM-ALIGNMENT.md`](./LLM-ALIGNMENT.md) rule A3 in both repositories (required status checks, branch protection, CODEOWNERS).
- Open the planned upstream PRs from the feasibility backlog (FCM transport profile, NIP-46 remote signing, `AppProfile` variant) so they are in flight before Phase 3 needs them.

Exit gate: two successful upstream synchronizations with no manual protocol or schema edits, and the A3 enforcement settings verified enabled.

### Phase 3 — Build the RustChat enterprise boundary

Implement external services and adapters for:

- Keycloak/OIDC identity lifecycle
- account provisioning, suspension, revocation, and recovery
- organization and group synchronization
- RustShare permission-aware RAG and MCP
- policy, retention, audit presentation, and export
- deployment, backup, restore, observability, and managed operations

Integrations use the verified upstream seams from [`FEASIBILITY.md`](./FEASIBILITY.md) (relay membership gate, NIP-OA attestation, NIP-IA identity archive, audit log, `/query` bridge). If a needed seam does not exist, the change goes upstream first — not into a private fork mechanism.

Exit gate: all contracts in [`CONTRACTS.md`](./CONTRACTS.md) have executable conformance tests, generated from machine-readable schemas per [`LLM-ALIGNMENT.md`](./LLM-ALIGNMENT.md) rule A8.

### Phase 4 — Rebrand and release clients

- Rebrand desktop, Android, and iOS applications.
- Web client: decided 2026-07-23 — RustChat tracks upstream. No separate web-client work; when `block/buzz` ships a web chat client it is rebranded and gated like the other clients ([`FEASIBILITY.md`](./FEASIBILITY.md)). Until then, supported surfaces are desktop and mobile, and no plan or marketing material may claim a RustChat web client.
- Change product identifiers, icons, update endpoints, privacy declarations, and store metadata.
- Preserve license and attribution notices. (Apache-2.0 grants no trademark rights to "Buzz"; rebranding is required for distribution.)
- Run the upstream client test suite plus RustChat branding and enterprise-login tests.
- iOS push requires a rebranded `AppProfile` variant upstream; Android push requires the upstream FCM transport profile. Neither is assumed complete until the upstream PRs land.

Exit gate: desktop and mobile clients pass the release matrix in [`TEST-STRATEGY.md`](./TEST-STRATEGY.md).

### Phase 5 — Internal dogfood and public preview

- Use RustChat internally as the primary Kubedo workspace.
- Exercise account suspension, device replacement, restore, upstream upgrades, agents, workflows, RustShare retrieval, desktop, and mobile.
- Do not advertise enterprise readiness before the defined reliability gates pass.

Exit gate: two production-like upgrade cycles and at least 30 days of internal usage without a critical architecture exception.

## Required documents

- [`00-final-pre-buzz-release.md`](./00-final-pre-buzz-release.md)
- [`SPEC-001-repository-and-upstream-boundaries.md`](./SPEC-001-repository-and-upstream-boundaries.md)
- [`SPEC-002-enterprise-control-plane.md`](./SPEC-002-enterprise-control-plane.md)
- [`CONTRACTS.md`](./CONTRACTS.md)
- [`TEST-STRATEGY.md`](./TEST-STRATEGY.md)
- [`FEASIBILITY.md`](./FEASIBILITY.md)
- [`LLM-ALIGNMENT.md`](./LLM-ALIGNMENT.md)
- [`PROMPT-LEDGER.md`](./PROMPT-LEDGER.md)
- [`prompts/P001-final-release-and-buzz-pivot-bootstrap.md`](./prompts/P001-final-release-and-buzz-pivot-bootstrap.md)

## Change-control rule

Any change that weakens an architecture rule above requires:

1. A new ADR.
2. A concrete compatibility impact assessment.
3. An upstream-sync impact assessment.
4. Contract and test updates in the same pull request.
5. Explicit approval from the architecture owner.

Convenience, implementation speed, or an LLM recommendation alone is not sufficient justification.
