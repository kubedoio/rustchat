# Frontend Architecture

Deep dive into the RustChat frontend architecture.

## Overview

The frontend is a Single Page Application (SPA) built with:
- **Framework:** Vue 3.5
- **Language:** TypeScript 5.9+
- **State:** Pinia 3+
- **Build:** Vite 7+
- **Styling:** Tailwind CSS 4+

## Directory Structure

```
frontend/src/
├── api/              # API client functions
│   ├── http/        # HTTP client (Fetch-based)
│   ├── client.ts    # Native API client
│   └── calls.ts     # Calls API client
├── components/       # Vue components
├── composables/      # Vue composables
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
├── router/          # Vue Router configuration
├── stores/          # Legacy Pinia stores
└── views/           # Page-level components
```

## Feature Module Pattern

Each feature follows a consistent structure:

```
features/[feature]/
├── repositories/    # Data access (API calls)
├── services/        # Business logic
├── stores/          # Pinia state
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
export const useChannelStore = defineStore('channels', () => {
  const channels = ref<Channel[]>([])
  
  async function fetchChannels() {
    channels.value = await channelRepository.getChannels()
  }
  
  return { channels, fetchChannels }
})
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

- Use `<script setup>` syntax
- Composition API preferred
- Props/Emits for component interface
- Composables for reusable logic

## Testing

- **Unit:** Vitest for composables/services
- **E2E:** Playwright for user flows
- **Contract:** HTTP client behavior tests

---

*See also: [Backend Architecture](./backend.md)*
