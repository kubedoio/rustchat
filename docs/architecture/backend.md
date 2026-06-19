# Backend Architecture

Deep dive into the RustChat backend architecture.

## Overview

The backend is built with:
- **Framework:** Axum 0.8
- **Runtime:** Tokio async runtime
- **Database:** PostgreSQL with SQLx
- **Cache/Events:** Redis

## Module Structure

```
backend/src/
├── a2a/              # Agent-to-agent communication layer
├── api/              # HTTP handlers (including v1/agents.rs)
│   ├── v1/          # Native API
│   └── v4/          # Mattermost-compatible API
├── auth/            # Authentication & JWT
├── config/          # Environment configuration
├── db/              # Database connection & migrations
├── error/           # Error types
├── jobs/            # Background workers (including sync & RAG ingestion)
├── mattermost_compat/  # MM compatibility utilities
├── middleware/      # Axum middleware
├── models/          # Data models (User with entity_type='agent', AgentConfig, Knowledge, etc.)
├── realtime/        # WebSocket hub
├── repositories/    # Database access repositories (AgentRepository, AgentFeedbackRepository, etc.)
├── services/        # Business logic (including llm/ completions and agents CRUD)
├── storage/         # S3 file storage & knowledge ingestion
└── telemetry/       # Logging & tracing
```

## Request Lifecycle

```
Request
  ↓
Middleware (auth, rate-limit, CORS, logging)
  ↓
Router
  ↓
Handler (extracts params, calls service)
  ↓
Service (business logic)
  ↓
Database / Storage / Cache
  ↓
Response (JSON serialization)
```

## Key Components

### API Layer

Two API surfaces:
- **v1:** Native API for first-party clients
- **v4:** Mattermost-compatible API for mobile/desktop clients

### Service Layer

One service per domain:
- `auth_service` - Authentication
- `channel_service` - Channel operations
- `post_service` - Message operations
- `file_service` - File uploads
- `user_service` - User management
- `agent_service` - AI Agents management and channel dispatcher logic
- `llm_service` - OpenAI connection, RAG prompting, and context truncation logic

### Database Access

- SQLx for compile-time checked queries
- Connection pooling via `sqlx::Pool`
- Migrations in `migrations/` directory

### WebSocket Hub

- Connection management
- Event fan-out
- Redis pub/sub for clustering
- Two endpoints: v1 (native) and v4 (Mattermost-compatible)

## Error Handling

```rust
pub enum AppError {
    Unauthorized,
    Forbidden,
    NotFound,
    Validation(String),
    Internal(String),
}
```

All errors convert to appropriate HTTP status codes.

## Authentication

- JWT tokens with configurable expiry
- Argon2id password hashing
- API key support for bots/integrations
- Optional SSO via OAuth/SAML

## Future Considerations

- Horizontal scaling with Redis clustering
- Read replicas for database
- CDN for file serving
- GraphQL API (v2)

---

*See also: [System Overview](./overview.md)*
