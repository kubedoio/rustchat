# Contributing to rustchat

Thank you for your interest in contributing to rustchat!

## Development Setup

1. Install Rust 1.75+ via [rustup](https://rustup.rs/)
2. Install Docker and Docker Compose
3. Clone the repository
4. Run `docker compose up -d` for dependencies
5. Copy `.env.example` to `.env`
6. Run `cargo build` in the `backend/` directory

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Write tests for new functionality
- Keep functions focused and small

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Commit with clear messages
6. Push and open a Pull Request

## Commit Messages

Follow conventional commits:

```
feat: add user registration endpoint
fix: correct JWT expiry calculation
docs: update API documentation
test: add channel permission tests
```

## Questions?

Open an issue or start a discussion.
