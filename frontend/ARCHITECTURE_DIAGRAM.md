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
│  │                     useMessageStore (1,060 lines)                 │   │
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
│  │                    messageService (242 lines)                      │   │
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
│        │  │  │              messageRepository (217 lines)          │ │
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
│  │              messageSocketHandlers.ts (171 lines)                   │ │
│  │                                                                     │ │
│  │  handlePost() ──────► messageService.handleIncomingMessage()       │ │
│  │  handlePostEdit() ──► messageService.handleMessageUpdate()         │ │
│  │  handleReaction() ──► messageService.handleReactionAdded()         │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                          CORE LAYER (Shared)                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                       │
│  │   Entities  │  │    Errors   │  │   Services  │                       │
│  │             │  │             │  │             │                       │
│  │    User     │  │  AppError   │  │  retry.ts   │                       │
│  │    Message  │  │   Network   │  │             │                       │
│  │    Channel  │  │    Error    │  │             │                       │
│  │    Call     │  │  NotFound   │  │             │                       │
│  │             │  │    Error    │  │             │                       │
│  │             │  │             │  │             │                       │
│  └─────────────┘  └─────────────┘  └─────────────┘                       │
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
   │Repository│   │Repository│   │  (none)  │
   │ Messages │   │ Channels │   │  Calls*  │
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

\* Calls is store-only: `features/calls/` has no repository or service
layer — the calls store (`features/calls/stores/callsStore.ts`) calls `api/calls.ts`
directly.

## File Size Targets vs Actual

| Layer | File | Target | Actual | Status |
|-------|------|--------|--------|--------|
| Entity | Message.ts | 50 | 89 | ⚠️ Over target |
| Entity | User.ts | 50 | 54 | ⚠️ Over target |
| Repository | messageRepository.ts | 200 | 217 | ⚠️ Over target |
| Service | messageService.ts | 250 | 242 | ✅ Good |
| Store | messageStore.ts | 300 | 1,060 | ⚠️ Over target |
| Handler | messageSocketHandlers.ts | 200 | 171 | ✅ Good |
| WebSocket | useWebSocket.ts (composable) | 200 | 1,019 | ⚠️ Over target |

## Key Principles

1. **No Layer Skipping**: Components → Store → Service → Repository → API
2. **Single Direction**: Data flows down, events flow up
3. **Feature Isolation**: Each feature has its own folder
4. **Pure Stores**: No business logic in stores
5. **Explicit Errors**: AppError hierarchy (`core/errors/AppError.ts`) with `errorUtils` helpers; repositories translate not-found to `null` and rethrow anything else
