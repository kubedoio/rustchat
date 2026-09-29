# Frontend Architecture Diagram

## Layered Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              UI LAYER                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │MessageList  │  │MessageInput │  │ThreadPanel  │  │  ChannelList    │ │
│  │   .vue      │  │   .vue      │  │   .vue      │  │    .vue         │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘ │
└─────────┼────────────────┼────────────────┼──────────────────┼──────────┘
          │                │                │                  │
          ▼                ▼                ▼                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           STORE LAYER (State Only)                       │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                     useMessageStore (270 lines)                   │   │
│  │  • messagesByChannel: Map<ChannelId, Message[]>                   │   │
│  │  • threadRepliesByRoot: Map<MessageId, Message[]>                 │   │
│  │  • loading, error, hasMoreOlder                                   │   │
│  │                                                                   │   │
│  │  Actions: setMessages, addMessage, updateMessage, removeMessage   │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          SERVICE LAYER (Business Logic)                  │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    messageService (225 lines)                     │   │
│  │                                                                   │   │
│  │  loadMessages(channelId) ─────┐                                   │   │
│  │  sendMessage(draft) ──────────┼──► Optimistic updates            │   │
│  │  loadThread(rootId) ──────────┼──► Deduplication                  │   │
│  │  addReaction(...) ────────────┼──► Error handling                 │   │
│  │  handleIncomingMessage(...) ◄─┘    Retry logic                    │   │
│  │                                                                   │   │
│  │  ┌─────────────────────────────────────────────────────────────┐  │   │
│  │  │  Orchestrates: Repository ◄────► Store ◄────► WebSocket     │  │   │
│  │  └─────────────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
          │
    ┌─────┴─────┐
    ▼           ▼
┌────────┐  ┌─────────────────────────────────────────────────────────┐
│  Store │  │                     REPOSITORY LAYER                     │
│ Updates│  │  ┌─────────────────────────────────────────────────────┐ │
│        │  │  │              messageRepository (206 lines)          │ │
│        │  │  │                                                      │ │
│        │  │  │  • findByChannel() ──► GET /api/channels/:id/posts  │ │
│        │  │  │  • create() ─────────► POST /api/posts              │ │
│        │  │  │  • update() ─────────► PUT /api/posts/:id           │ │
│        │  │  │  • delete() ─────────► DELETE /api/posts/:id        │ │
│        │  │  │  • addReaction() ────► POST /api/posts/:id/react    │ │
│        │  │  │                                                      │ │
│        │  │  │  Responsibilities:                                   │ │
│        │  │  │  • API communication                                 │ │
│        │  │  │  • Data mapping (API ↔ Domain)                       │ │
│        │  │  │  • Retry logic                                       │ │
│        │  │  │  • Caching (future)                                  │ │
│        │  │  └─────────────────────────────────────────────────────┘ │
│        │  └─────────────────────────────────────────────────────────┘
│        │                          │
│        │                          ▼
│        │  ┌─────────────────────────────────────────────────────────┐
│        │  │                      API CLIENTS                        │
│        │  │              (postsApi, channelsApi, etc)               │
│        │  └─────────────────────────────────────────────────────────┘
│        │                          │
│        └──────────────────────────┤
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      WEBSOCKET LAYER (Real-time)                         │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                   useWebSocket.ts composable (realtime layer)      │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │ │
│  │  │  connect()  │  │ sendMessage()│  │  onEvent()  │  │ connected  │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘ │ │
│  │                                                                    │ │
│  │  Event Routing:                                                    │ │
│  │  'posted' ───────► messageSocketHandlers.ts                        │ │
│  │  'post_edited' ──► messageSocketHandlers.ts                        │ │
│  │  'reaction_added'► messageSocketHandlers.ts                        │ │
│  │  'user_updated' ─► userSocketHandlers.ts (future)                  │ │
│  │  'call_started' ─► callSocketHandlers.ts (future)                  │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                            │                                            │
│                            ▼                                            │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │              messageSocketHandlers.ts (156 lines)                   │ │
│  │                                                                     │ │
│  │  handlePost() ──────► messageService.handleIncomingMessage()       │ │
│  │  handlePostEdit() ──► messageService.handleMessageUpdate()         │ │
│  │  handleReaction() ──► messageService.handleReactionAdded()         │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                          CORE LAYER (Shared)                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────┐  │
│  │   Entities  │  │    Types    │  │    Errors   │  │  Repositories  │  │
│  │  ┌───────┐  │  │  ┌──────┐   │  │  ┌──────┐   │  │   Interface    │  │
│  │  │ User  │  │  │  │Result│   │  │  │AppError│  │  │                │  │
│  │  ├───────┤  │  │  ├──────┤   │  │  ├──────┤   │  │  Repository<T> │  │
│  │  │Message│  │  │  │Async │   │  │  │Network │  │  │    findById    │  │
│  │  ├───────┤  │  │  │Result│   │  │  │  Error │  │  │    findAll     │  │
│  │  │Channel│  │  │  └──────┘   │  │  ├──────┤   │  │    create      │  │
│  │  ├───────┤  │  │             │  │  │NotFound│  │  │    update      │  │
│  │  │ Call  │  │  │             │  │  │ Error  │  │  │    delete      │  │
│  │  └───────┘  │  │             │  │  └──────┘   │  │                │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

## Dependency Flow

```
                    ┌──────────┐
                    │   API    │
                    └────┬─────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
   ┌──────────┐   ┌──────────┐   ┌──────────┐
   │Repository│   │Repository│   │Repository│
   │ Messages │   │ Channels │   │   Calls  │
   └────┬─────┘   └────┬─────┘   └────┬─────┘
        │              │              │
        └──────────────┼──────────────┘
                       ▼
              ┌────────────────┐
              │    Services    │
              │ (Orchestration)│
              └───────┬────────┘
                      │
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
   ┌────────┐   ┌────────┐   ┌──────────┐
   │ Store  │   │ Store  │   │  Store   │
   │Messages│   │Channels│   │   Calls  │
   └───┬────┘   └───┬────┘   └────┬─────┘
       │            │             │
       └────────────┼─────────────┘
                    ▼
            ┌──────────────┐
            │  Components  │
            └──────────────┘
```

## File Size Targets vs Actual

| Layer | File | Target | Actual | Status |
|-------|------|--------|--------|--------|
| Entity | Message.ts | 50 | 80 | ✅ Good |
| Entity | User.ts | 50 | 42 | ✅ Good |
| Repository | messageRepository.ts | 200 | 206 | ✅ Good |
| Service | messageService.ts | 250 | 225 | ✅ Good |
| Store | messageStore.ts | 300 | 270 | ✅ Good |
| Handler | messageSocketHandlers.ts | 200 | 156 | ✅ Good |
| WebSocket | useWebSocket.ts (composable) | 200 | 999 | ⚠️ Over target |

## Key Principles

1. **No Layer Skipping**: Components → Store → Service → Repository → API
2. **Single Direction**: Data flows down, events flow up
3. **Feature Isolation**: Each feature has its own folder
4. **Pure Stores**: No business logic in stores
5. **Explicit Errors**: Result types for error handling
