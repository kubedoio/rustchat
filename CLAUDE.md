# Project: RustChat

RustChat is a high-performance, secure collaboration platform built with Rust and Vue.

## Health Stack

- backend-lint: cd backend && cargo clippy --all-targets -- -D warnings
- backend-test: cd backend && cargo test
- frontend-typecheck: cd frontend && npx vue-tsc --noEmit
- frontend-test: cd frontend && npm run test:unit
- frontend-deadcode: cd frontend && npx knip
- push-proxy-lint: cd push-proxy && cargo clippy --all-targets -- -D warnings
- push-proxy-test: cd push-proxy && cargo test
