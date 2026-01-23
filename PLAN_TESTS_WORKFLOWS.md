# Test Strategy and Workflow Plan for Rustchat

This document outlines the draft plan for implementing Continuous Integration (CI) workflows and Functional/End-to-End (E2E) testing for the Rustchat platform.

## 1. CI/CD Workflows

We will implement GitHub Actions workflows to ensure code quality and stability on every pull request and push to the main branch.

### Backend Workflow (`.github/workflows/backend-ci.yml`)

This workflow focuses on the Rust backend.

*   **Triggers:**
    *   `push` to `main`
    *   `pull_request` targeting `main` (paths: `backend/**`, `.github/workflows/backend-ci.yml`)

*   **Jobs:**
    *   **Test & Lint:**
        *   **OS:** `ubuntu-latest`
        *   **Services:** PostgreSQL (latest) - Required for integration tests.
        *   **Steps:**
            1.  Checkout code.
            2.  Install Rust toolchain (stable).
            3.  Cache Cargo registry and build artifacts.
            4.  Start PostgreSQL service.
            5.  Run `sqlx migrate run` (requires `sqlx-cli`).
            6.  **Check:** `cargo check`
            7.  **Format:** `cargo fmt --all -- --check`
            8.  **Lint:** `cargo clippy -- -D warnings`
            9.  **Test:** `cargo test` (Runs unit and integration tests).

### Frontend Workflow (`.github/workflows/frontend-ci.yml`)

This workflow focuses on the Vue 3 frontend.

*   **Triggers:**
    *   `push` to `main`
    *   `pull_request` targeting `main` (paths: `frontend/**`, `.github/workflows/frontend-ci.yml`)

*   **Jobs:**
    *   **Build & Check:**
        *   **OS:** `ubuntu-latest`
        *   **Steps:**
            1.  Checkout code.
            2.  Setup Node.js (LTS).
            3.  Cache `node_modules`.
            4.  Install dependencies: `npm ci`.
            5.  **Type Check & Build:** `npm run build` (This runs `vue-tsc` internally).
            6.  **Lint:** (If ESLint/Prettier is configured) `npm run lint`.

## 2. Functional Testing Strategy

Functional tests will verify that the system works as intended from a user's perspective or API consumer's perspective.

### Backend Integration Tests

We will add a dedicated `tests/` directory in `backend/` to house integration tests that spin up the application and hit real HTTP endpoints backed by a test database.

*   **Location:** `backend/tests/`
*   **Tooling:**
    *   `tokio::test`: To run async tests.
    *   `reqwest`: HTTP client to make requests to the API.
    *   `sqlx`: To manage test database state (transactions/migrations).
    *   `uuid`: For generating unique test data.

*   **Test Architecture:**
    *   **`TestApp` Helper:** A struct/helper to:
        *   Initialize a localized `PgPool`.
        *   Apply migrations (or ensure schema exists).
        *   Start the Axum server on a random port.
        *   Provide client methods for common actions (e.g., `post_login`).

*   **Key Scenarios:**
    1.  **Health Check:** Verify `/api/v1/health` returns 200.
    2.  **Authentication:**
        *   Register a new user (success/failure).
        *   Login (verify JWT token receipt).
    3.  **Teams & Channels:**
        *   Create a team.
        *   Create a channel within a team.
    4.  **Messaging:**
        *   Post a message to a channel.
        *   Retrieve messages from a channel.

### Frontend E2E Tests

We will use **Playwright** for End-to-End testing of the frontend application. Playwright is chosen for its speed, reliability, and modern browser support.

*   **Location:** `frontend/e2e/` (or root `e2e/` if we want to separate it completely, but keeping it in `frontend` is standard).
*   **Tooling:** `@playwright/test`

*   **Setup:**
    *   Install Playwright: `npm init playwright@latest`.
    *   Configure `playwright.config.ts`.

*   **CI Integration:**
    *   Add a job to `frontend-ci.yml` or a separate `e2e.yml` to run `npx playwright test`.
    *   This requires starting the backend and frontend servers before running tests.

*   **Key Scenarios:**
    1.  **Public Routes:** Verify landing page loads.
    2.  **Login Flow:**
        *   Navigate to login.
        *   Enter credentials.
        *   Verify redirection to the dashboard.
    3.  **Messaging UI:**
        *   Select a channel.
        *   Type in the message box.
        *   Send message.
        *   Verify message appears in the list.

## 3. Implementation Plan (Next Steps)

1.  **Workflows:** Create the `.github/workflows` directory and the YAML files defined above.
2.  **Backend Tests:**
    *   Create `backend/tests/common/mod.rs` (Test helpers).
    *   Create `backend/tests/api_health.rs`.
    *   Create `backend/tests/api_auth.rs`.
3.  **Frontend Tests:**
    *   Run Playwright initialization in `frontend/`.
    *   Write a basic `example.spec.ts` for the login page.
