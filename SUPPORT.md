# Support Policy

## Community Support (Free)

RustChat is under active development. Community support is available through:

- **GitHub Discussions**: Ask questions, share setups, and get help from the community.
- **GitHub Issues**: Report bugs, request features, or suggest documentation improvements.
- **Documentation**: See the [docs/](docs/) directory for deployment, development, and architecture guides.

We monitor discussions and issues regularly, but response times are best-effort.

## Supported Versions

RustChat is currently pre-1.0 and under active development.

| Version | Support Level |
|---------|---------------|
| `main` branch | Active development — latest fixes and features |
| Latest tagged release | Supported — receives critical fixes |
| Older releases | Not supported — upgrade to latest |

Before 1.0, minor version bumps may include breaking changes. Always review [CHANGELOG.md](CHANGELOG.md) before upgrading.

## Nightly Builds

Nightly container images are published automatically from the `main` branch. They are useful for testing the latest changes but are not guaranteed to be stable. See [docs/release-process.md](docs/release-process.md) for tag conventions.

## Reporting Security Issues

**Do not open public issues for security vulnerabilities.**

See [SECURITY.md](SECURITY.md) for responsible disclosure instructions and [MAINTAINERS.md](MAINTAINERS.md) for security contacts.

## Commercial Support

Community support is handled through GitHub Discussions and Issues.

## Asking Good Questions

To get help faster:

1. Search existing discussions and issues first
2. Include your RustChat version, deployment method (Docker, bare metal), and relevant environment variables (redact secrets)
3. Provide steps to reproduce for bugs
4. Include logs (redact tokens and passwords)
