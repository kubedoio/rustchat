# RustChat Governance

## Overview

RustChat is an open-source project maintained by its community of contributors.
This document describes how the project is governed today and how decisions are
made.

## Current State

The project is currently maintained by a small core team with community
contributions. As the project matures, governance may evolve to include a
technical steering committee or similar structure.

## Roles

### Maintainers

Maintainers have write access to the repository and are responsible for:

- Reviewing and merging pull requests
- Triaging issues
- Cutting releases
- Enforcing the code of conduct
- Setting technical direction

See [MAINTAINERS.md](MAINTAINERS.md) for the current list.

### Contributors

Anyone who submits a pull request, reports an issue, or improves documentation
is a contributor. Contributors are recognized in release notes and commit
history.

### Reviewers

Reviewers are domain experts who review pull requests in specific areas (for
example Mattermost compatibility, security, and architecture). They are listed
in [CODEOWNERS](.github/CODEOWNERS).

## Decision Making

### Day-to-Day Changes

- Most changes are decided through pull request review.
- Every merged PR requires an independent human approval unless an explicitly
  documented emergency process says otherwise.
- Elevated and architectural changes additionally require maintainer
  sponsorship/sign-off and the evidence defined by the corresponding risk tier.
- The PR author does not count as the independent reviewer.
- For the highest-risk architectural changes (authentication/security model,
  storage/data model, protocol compatibility, repository governance), seek a
  second independent review when the active reviewer pool permits. This is a
  target, not a falsely unenforceable hard gate with the current small team.
- `.governance/risk-tiers.yml` is the machine-readable source for the current
  review/evidence policy.

### Architectural Decisions

New significant technical decisions are recorded as Architecture Decision
Records (ADRs) under **`docs/adr/`**, whose index is
[docs/adr/README.md](docs/adr/README.md).

`docs/decisions/` is retained only for backward-compatible links/indexes. Do
not create new ADRs there.

An ADR is required for:

- New major dependencies
- Database/storage model changes
- Authentication or security model changes
- API/protocol compatibility breaking changes
- Infrastructure or deployment model changes
- Repository-wide governance/security model changes

Short-form decision notes may still be used for elevated changes that do not
meet the ADR threshold.

### Conflict Resolution

If reviewers disagree on a change:

1. Discuss in the pull request with technical arguments.
2. If unresolved, escalate to maintainers listed in [MAINTAINERS.md](MAINTAINERS.md).
3. Maintainers make a binding decision with a written rationale.

## Pull Request Size and Risk

`.governance/pr-size-limits.yml` provides hard size guidance for standard and
elevated work so routine PRs remain reviewable.

Architectural changes are different: `.governance/risk-tiers.yml` intentionally
does not impose an arbitrary hard file/line ceiling. They still must be
coherent, reviewable, and split when independent decisions can be separated.
Large architectural PRs require independent review, maintainer sponsorship, and
design evidence rather than mechanical line slicing.

A contributor must not relabel ordinary feature work as "architectural" merely
to bypass standard/elevated size limits.

## Repository Integrity Program

ADR-006 defines a staged repository-integrity program. Its machine-readable
contracts begin in `planned` state and become `active` only after implementation
and objective evidence.

The program does not override agent path boundaries or existing human-review
requirements.

## Machine-Readable Governance Files

The `.governance/` directory contains YAML files used by maintainer automation
and CI tooling, including:

- `agent-contracts.yml`
- `risk-tiers.yml`
- `protected-paths.yml`
- `pr-size-limits.yml`
- `repository-integrity-contracts.yml`

These files are policy. Changes to them are architectural and require the
corresponding human review.

## Contributing

All contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the
workflow.

All contributions must be signed off per the [DCO](DCO.md).

## Code of Conduct

RustChat follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
Violations should be reported to the maintainers.

## Roadmap

The public roadmap is maintained in [ROADMAP.md](ROADMAP.md).

## License

RustChat is released under the [Apache-2.0 License](LICENSE).
