# Migration Guide - Component Updates

## Overview
This guide helps migrate Vue components from the old store-based architecture to the new feature-based architecture.

---

## Phase 1: Import Updates (Safe, can be done incrementally)

### Messages
```typescript
// OLD
import { useMessagesStore } from '@/stores/messages'
const messagesStore = useMessagesStore()
await messagesStore.loadMessages(channelId)

// NEW
import { messageService } from '@/features/messages'
import { useMessageStore } from '@/features/messages'
const messageStore = useMessageStore()
await messageService.loadMessages(channelId)
```

### Calls
```typescript
// OLD
import { useCallsStore } from '@/stores/calls'
const callsStore = useCallsStore()
await callsStore.startCall(channelId)

// NEW
import { useCallsStore } from '@/features/calls'
const callsStore = useCallsStore()
await callsStore.startCall(channelId)
```

### Channels
```typescript
// OLD
import { useChannelStore } from '@/stores/channels'
const channelStore = useChannelStore()
await channelStore.fetchChannels(teamId)
channelStore.selectChannel(channelId)

// NEW
import { channelService } from '@/features/channels'
import { useChannelStore } from '@/features/channels'
const channelStore = useChannelStore()
await channelService.loadChannels(teamId)
channelService.selectChannel(channelId)
```

---

## Phase 2: Store Access Patterns

### Reading State (same pattern)
```typescript
// Both old and new
const currentChannel = computed(() => channelStore.currentChannel)
const messages = computed(() => messageStore.getMessages(channelId))
```

### Writing State (now via service)
```typescript
// OLD - Store handled everything
await messagesStore.sendMessage({ content: 'Hello' })

// NEW - Service orchestrates
await messageService.sendMessage({ channelId, content: 'Hello' })
```

---

## Phase 3: WebSocket Updates

### Old Way (centralized)
```typescript
import { useWebSocket } from '@/composables/useWebSocket'
const { onEvent } = useWebSocket()

onEvent('posted', (data) => {
  // Handle in component
})
```

### New Way (feature-specific, usually not needed in components)
```typescript
// WebSocket handlers are now in features/[feature]/handlers/
// Components usually don't need to register handlers directly
// Just use the reactive store data

import { useMessageStore } from '@/features/messages'
const messageStore = useMessageStore()

// Data is automatically updated via WebSocket handlers
const messages = computed(() => messageStore.getMessages(channelId))
```

### If you need custom WebSocket handling
```typescript
import { wsManager } from '@/core/websocket/WebSocketManager'

// Register handler
const unsubscribe = wsManager.on('custom_event', (event) => {
  console.log('Received:', event)
})

// Cleanup
onUnmounted(() => unsubscribe())
```

---

## Phase 4: Complete Component Example

### Before (Old Style)
```vue
<template>
  <div class="channel-view">
    <MessageList :messages="messagesStore.messages" />
    <MessageInput @send="sendMessage" />
  </div>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { useMessagesStore } from '@/stores/messages'
import { useChannelStore } from '@/stores/channels'
import { useWebSocket } from '@/composables/useWebSocket'
import MessageList from './MessageList.vue'
import MessageInput from './MessageInput.vue'

const props = defineProps<{ channelId: string }>()

const messagesStore = useMessagesStore()
const channelStore = useChannelStore()
const { onEvent } = useWebSocket()

const messages = computed(() => messagesStore.messages)

// Load messages when channel changes
watch(() => props.channelId, async (id) => {
  if (id) {
    await messagesStore.loadMessages(id)
  }
}, { immediate: true })

// Send message
async function sendMessage(content: string) {
  await messagesStore.sendMessage({ 
    channel_id: props.channelId, 
    content 
  })
}

// Handle incoming WebSocket messages
onEvent('posted', (data) => {
  const post = JSON.parse(data.post)
  if (post.channel_id === props.channelId) {
    messagesStore.handleNewMessage(post)
  }
})
</script>
```

### After (New Style)
```vue
<template>
  <div class="channel-view">
    <MessageList :messages="messages" />
    <MessageInput @send="sendMessage" />
  </div>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { useMessageStore, messageService } from '@/features/messages'
import MessageList from './MessageList.vue'
import MessageInput from './MessageInput.vue'

const props = defineProps<{ channelId: string }>()

const messageStore = useMessageStore()

// Get messages from store (updated automatically by WebSocket handlers)
const messages = computed(() => messageStore.getMessages(props.channelId))

// Load messages when channel changes
watch(() => props.channelId, async (id) => {
  if (id) {
    await messageService.loadMessages(id)
  }
}, { immediate: true })

// Send message (service handles optimistic update + WebSocket sync)
async function sendMessage(content: string) {
  await messageService.sendMessage({ 
    channelId: props.channelId, 
    content 
  })
}

// Note: No WebSocket handling needed - it's done in features/messages/handlers/
</script>
```

---

## Common Patterns

### Loading States
```typescript
// OLD
const loading = computed(() => messagesStore.loading)

// NEW
const loading = computed(() => messageStore.loading)
```

### Error Handling
```typescript
// OLD
const error = computed(() => messagesStore.error)

// NEW
const error = computed(() => messageStore.error)
// Or use service which throws
```

### Pagination (Loading More)
```typescript
// OLD
await messagesStore.loadMoreMessages(channelId)

// NEW
await messageService.loadOlderMessages(channelId)
```

---

## Quick Reference: Import Mapping

| Old Import | New Import |
|------------|------------|
| `useMessagesStore` from `@/stores/messages` | `useMessageStore` from `@/features/messages` |
| `useCallsStore` from `@/stores/calls` | `useCallsStore` from `@/features/calls` (path only; store id and export name unchanged) |
| `useChannelStore` from `@/stores/channels` | `useChannelStore` from `@/features/channels` |
| `useWebSocket` from `@/composables/useWebSocket` | `useWebSocket` from `@/composables/useWebSocketAdapter` (temp) |
| | `wsManager` from `@/core/websocket/WebSocketManager` (preferred) |

---

## Testing During Migration

### Feature Flag Approach
```typescript
const useNewFeatures = import.meta.env.VITE_USE_NEW_FEATURES === 'true'

const messageStore = useNewFeatures 
  ? useMessageStore()  // new
  : useMessagesStore() // old
```

### Gradual Component Migration
1. Pick one component at a time
2. Update imports
3. Replace store calls with service calls
4. Test thoroughly
5. Move to next component

---

## Deprecation Timeline

1. **Week 1-2**: New features available alongside old stores
2. **Week 3-4**: Migrate high-priority components
3. **Week 5-6**: Migrate remaining components
4. **Week 7**: Add deprecation warnings to old stores
5. **Week 8**: Remove old stores
