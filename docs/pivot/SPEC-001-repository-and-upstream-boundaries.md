# SPEC-001: Repository and Upstream Boundaries

**Status:** Proposed
**Owner:** RustChat architecture owner
**Related ADR:** ADR-004

## Objective

Make architectural drift difficult by construction. Buzz source ownership, RustChat enterprise ownership, and legacy ownership must be mechanically and organizationally separate.

## Repository responsibilities

### `block/buzz`

Authoritative upstream for:

- event and relay protocol
- channels, threads, DMs, presence, and messaging behavior
- workflow and agent substrate
- Git and project collaboration
- desktop, web, and Flutter client foundations
- generic search, audit, media, and huddle behavior

### `kubedoio/buzz`

Tracked downstream fork. Allowed changes:

- product names, icons, colors, bundle identifiers, and store metadata
- release/update endpoints
- configuration injection points
- generic fixes awaiting upstream acceptance
- generic extension hooks that are proposed upstream
- temporary compatibility patches documented in the patch ledger

Forbidden without a dedicated accepted ADR:

- private RustChat event kinds replacing upstream semantics
- downstream-only relay protocol behavior
- edits to already-published upstream migrations
- RustChat customer/business logic inside Buzz crates
- direct calls from Buzz core into RustChat enterprise services
- replacing NIP authentication semantics with an incompatible private protocol

### `kubedoio/rustchat`

Owns:

- distribution manifests and version matrix
- enterprise identity and account lifecycle
- Keycloak deployment and configuration
- organization, group, policy, and subscription control plane
- RustShare and company-memory integration
- retention, export, compliance presentation, and support tooling
- deployment, backup, restore, monitoring, and release orchestration
- contracts and end-to-end conformance tests

## Target directory structure

```text
rustchat/
  docs/
    adr/
    pivot/
  contracts/
    identity/
    lifecycle/
    rustshare/
    distribution/
  services/
    identity-controller/
    organization-controller/
    rustshare-bridge/
    compliance-exporter/
  deploy/
    compose/
    helm/
    managed/
  distribution/
    versions.yaml
    branding/
    release/
  tests/
    conformance/
    e2e/
    upgrade/
  scripts/
    pivot/
    release/
```

Buzz source must not exist as a normal tracked directory in this repository.

Permitted references to the Kubedo Buzz fork:

- immutable container digest
- immutable Git commit or tag
- Cargo Git dependency pinned to a commit when a library dependency is justified
- Git submodule using a commit pin
- release metadata in `distribution/versions.yaml`

## Upstream synchronization model

The Kubedo fork must use:

```text
origin   https://github.com/kubedoio/buzz.git
upstream https://github.com/block/buzz.git
```

Required branches:

- `main`: current tested Kubedo integration branch
- `upstream-main`: optional mirror of upstream main
- `release/rustchat-X.Y`: stabilized RustChat distribution branch

Recommended sync sequence:

1. Fetch upstream.
2. Rebase or merge upstream main into a temporary sync branch.
3. Generate a changed-upstream report.
4. Run upstream tests unchanged.
5. Reapply or verify downstream patches.
6. Run RustChat conformance tests.
7. Open a reviewed sync pull request.
8. Update the pinned Buzz SHA only after all gates pass.

No unattended job may move a production RustChat pin directly to a new upstream commit.

## Downstream patch ledger

`kubedoio/buzz` must contain a machine-readable and human-readable patch ledger. Each patch entry requires:

- identifier
- affected files
- reason
- architecture category
- owner
- upstream issue or PR URL
- date introduced
- test coverage
- removal condition
- current status

Patch classes:

- `branding`
- `packaging`
- `extension-hook`
- `upstream-pending`
- `temporary-workaround`

A `core-divergence` class is forbidden unless authorized by an accepted ADR.

Patch budget:

- target: fewer than 20 active downstream patches
- warning: 20–35
- architecture review required: more than 35
- pivot re-evaluation required: more than 50

## Dependency rules

Enterprise services may depend on stable or deliberately selected interfaces such as:

- Buzz protocol/event types
- Buzz SDK
- supported WebSocket/HTTP APIs
- MCP and CLI contracts
- published events and webhooks

Enterprise services must not depend directly on:

- Buzz database tables
- private relay modules
- unversioned internal functions
- desktop local-storage implementation details
- unpublished migration ordering

Direct database access across the boundary is prohibited.

## Data ownership

- Buzz owns collaboration event data and its projections.
- Keycloak owns enterprise authentication identities and credentials.
- RustChat identity controller owns the binding and lifecycle state between enterprise identity and Buzz identity.
- RustShare owns durable knowledge artifacts and retrieval indexes.
- RustChat compliance services may read through supported export/query contracts but must not mutate Buzz-owned data directly.

## Version compatibility

Every RustChat release records:

- upstream Buzz repository
- upstream Buzz commit SHA
- Kubedo Buzz commit SHA
- desktop version
- mobile version
- protocol/conformance version
- RustChat enterprise-service versions
- Keycloak version
- RustShare contract version

A RustChat release is one tested version matrix, not an arbitrary combination of latest components.

## Architecture exceptions

An exception request must include:

1. Why no supported interface is sufficient.
2. Exact upstream files or semantics affected.
3. Compatibility and upgrade consequences.
4. A removal plan.
5. Tests proving no unauthorized behavior change.
6. An ADR when protocol, identity, authorization, schema, or data ownership is affected.
