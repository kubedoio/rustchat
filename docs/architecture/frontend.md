# Frontend Architecture

Deep dive into the RustChat frontend architecture.

## Overview

The frontend is a Single Page Application (SPA) built with:
- **Framework:** Svelte 5
- **Language:** TypeScript 5.9+
- **State:** Svelte stores
- **Build:** Vite 7+
- **Styling:** Tailwind CSS 4+

## Directory Structure

```
frontend/src/
├── api/              # API client functions
│   ├── http/        # HTTP client (Fetch-based)
│   ├── client.ts    # Native API client
│   └── calls.ts     # Calls API client
├── components/       # Svelte components
├── shared/          # Shared utilities and helpers
├── core/            # Shared primitives
│   ├── entities/    # Domain entities
│   ├── errors/      # Error types
│   └── websocket/   # WebSocket infrastructure
├── features/        # Domain feature modules
│   ├── auth/
│   ├── calls/
│   ├── channels/
│   ├── messages/
│   └── ...
├── svelte/          # Svelte-specific infrastructure
│   └── Router.svelte # Custom SPA router
├── stores/          # Legacy stores (being migrated to Svelte stores)
└── views/           # Page-level components
```

## Feature Module Pattern

Each feature follows a consistent structure:

```
features/[feature]/
├── repositories/    # Data access (API calls)
├── services/        # Business logic
├── stores/          # Svelte stores
├── handlers/        # WebSocket event handlers
├── components/      # Feature-specific components
└── index.ts         # Public API
```

## State Management

### Legacy Stores (deprecated)
```
stores/
├── auth.ts
├── channels.ts
├── messages.ts
└── ...
```

### Modern Feature Stores (recommended)
```typescript
// features/channels/stores/channelStore.ts
import { writable } from 'svelte/store'

function createChannelStore() {
  const { subscribe, set, update } = writable<Channel[]>([])

  return {
    subscribe,
    async fetchChannels() {
      const channels = await channelRepository.getChannels()
      set(channels)
    }
  }
}

export const channelStore = createChannelStore()
```

## Data Flow

```
User Action
  ↓
Component
  ↓
Service (business logic)
  ↓
Repository (API call)
  ↓
HTTP Client
  ↓
Backend API
```

WebSocket events flow in reverse:
```
WebSocket Message
  ↓
Handler
  ↓
Service
  ↓
Store Update
  ↓
Component Re-render
```

## HTTP Client

Custom Fetch-based HTTP client with:
- Request/response interceptors
- Auth token injection
- ID normalization (Mattermost ↔ UUID)
- Error handling
- Upload progress support

## WebSocket Integration

Native WebSocket client for real-time updates:
- Auto-reconnect
- Heartbeat/ping
- Event subscription management

## Component Guidelines

- Use `<script lang="ts">` with Svelte 5 runes
- Runes API preferred ($state, $derived, $effect)
- Props/$props for component interface
- Shared utilities for reusable logic

## Testing

- **Unit:** Vitest for composables/services
- **E2E:** Playwright for user flows
- **Contract:** HTTP client behavior tests

---

*See also: [Backend Architecture](./backend.md)*
