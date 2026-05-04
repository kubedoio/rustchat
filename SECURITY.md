# Security Policy

## Security Philosophy

Rustchat is security-sensitive software. We rely on open development, careful review, and responsible disclosure to reduce risk for operators and users.

If you find a vulnerability, please report it privately so a fix can be prepared before public disclosure.

## Supported Versions

RustChat is currently pre-1.0 and under active development.

| Version | Supported |
| ------- | --------- |
| `main` (current development) | :white_check_mark: |
| Latest tagged release | :white_check_mark: |
| Older tags/releases | :x: |

Before 1.0, minor version bumps may include breaking changes. Always review [CHANGELOG.md](CHANGELOG.md) before upgrading.

## Dependency Security Process

- Dependencies are monitored by Dependabot for automated update PRs
- `cargo audit` runs in CI to detect known vulnerable Rust crates
- `npm audit` runs in CI for frontend dependencies
- Security-related dependency updates are prioritized and released as patch versions when applicable

## Nightly Stability Disclaimer

Nightly builds (`nightly` container tags) are built automatically from `main` and are not guaranteed to be stable. They are intended for testing and feedback only.

## Reporting a Vulnerability

**Do not open public GitHub issues for security vulnerabilities.**

Use GitHub Private Vulnerability Reporting:

1. [Open a private advisory](https://github.com/rustchatio/rustchat/security/advisories/new)
2. Include enough detail for reproduction and impact assessment

If private advisory submission is unavailable for you, contact the maintainers listed in [MAINTAINERS.md](MAINTAINERS.md) through a private channel on GitHub and include `SECURITY:` in the subject/title.

### What to Include

Please provide:

1. Description of the vulnerability
2. Affected component(s) and version/commit
3. Reproduction steps or proof-of-concept
4. Expected impact and attack preconditions
5. Any suggested mitigations

## Response Process

1. **Acknowledgment:** We aim to acknowledge reports within 3 business days.
2. **Triage:** We validate and assess severity/impact.
3. **Remediation:** We prepare and test a fix.
4. **Disclosure:** We coordinate disclosure after a patch or mitigation is available.

We will credit reporters unless anonymity is requested.

## Deployment Hardening References

For production hardening guidance, see:

- `docs/security-deployment-guide.md`
- `docs/security-zero-trust-guide.md`

These documents cover secure configuration, token transport, TLS, security headers, and operational safeguards.
