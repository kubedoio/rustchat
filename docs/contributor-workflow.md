# Contributor Workflow

This document describes the GitHub-based contribution workflow for RustChat. For development setup, coding guidelines, and code quality requirements, see [CONTRIBUTING.md](../CONTRIBUTING.md).

## Fork and Branch

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rustchat.git
   cd rustchat
   ```
3. Create a feature branch from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   ```

## Commits

All commits must be signed off per the [DCO](../DCO.md):

```bash
git commit -s -m "feat: add user search endpoint"
```

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation only
- `test:` — Tests
- `refactor:` — Code change that neither fixes a bug nor adds a feature
- `chore:` — Maintenance tasks

## Tests

Before opening a PR:

```bash
# Backend
cd backend && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib

# Frontend
cd frontend && npm ci --ignore-scripts && npm run apply:dependency-patches && npm run build && npm run test:unit
```

## Pull Request

1. Push your branch to your fork
2. Open a pull request against `main`
3. Fill in the PR template completely
4. Ensure all CI checks pass
5. Address review comments
6. Maintain a clean commit history (rebase if requested)

## Review Expectations

- Standard changes: 1 approval
- Architectural or compatibility changes: 2 approvals, including CODEOWNERS reviewers
- All checks must pass before merge
- No direct pushes to `main`

## Good First Issues

Look for issues labeled [`good-first-issue`](https://github.com/rustchatio/rustchat/labels/good-first-issue). These are:

- Small and well-defined
- Have clear acceptance criteria
- Do not touch risky behavior (auth, permissions, payment-like flows)

## Getting Help

- **Architecture questions**: [Architecture Guide](architecture.md)
- **Development setup**: [Development Guide](development.md)
- **Compatibility**: [Compatibility Scope](compatibility.md)
- **General questions**: GitHub Discussions
