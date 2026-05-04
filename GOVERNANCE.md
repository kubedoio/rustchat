# RustChat Governance

## Overview

RustChat is an open-source project maintained by its community of contributors. This document describes how the project is governed today and how decisions are made.

## Current State

The project is currently maintained by a small core team with community contributions. As the project matures, governance may evolve to include a technical steering committee or similar structure.

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

Anyone who submits a pull request, reports an issue, or improves documentation is a contributor. Contributors are recognized in release notes and commit history.

### Reviewers

Reviewers are domain experts who review pull requests in specific areas (e.g., Mattermost compatibility, security). They are listed in [CODEOWNERS](.github/CODEOWNERS).

## Decision Making

### Day-to-Day Changes

- Most changes are decided through pull request review
- Two approvals are required for architectural changes
- One approval is sufficient for standard changes (docs, tests, UI polish)

### Architectural Decisions

Significant technical decisions are recorded as Architecture Decision Records (ADRs) in `docs/decisions/` or `docs/adr/`.

An ADR is required for:
- New major dependencies
- Database schema changes
- Authentication or security model changes
- API compatibility breaking changes
- Infrastructure or deployment model changes

### Conflict Resolution

If reviewers disagree on a change:
1. Discuss in the pull request with technical arguments
2. If unresolved, escalate to the maintainers listed in [MAINTAINERS.md](MAINTAINERS.md)
3. Maintainers make a binding decision with a written rationale

## Machine-Readable Governance Files

The `.governance/` directory contains YAML files (`agent-contracts.yml`, `risk-tiers.yml`, `protected-paths.yml`, `pr-size-limits.yml`) used by maintainer automation and CI tooling. **Normal contributors do not need to read or understand these files.** They are for internal process enforcement and do not change the contribution workflow described in [CONTRIBUTING.md](CONTRIBUTING.md).

## Contributing

All contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow.

All contributions must be signed off per the [DCO](DCO.md).

## Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Violations should be reported to the maintainers.

## Roadmap

The public roadmap is maintained in [ROADMAP.md](ROADMAP.md).

## License

RustChat is released under the [Apache-2.0 License](LICENSE).
