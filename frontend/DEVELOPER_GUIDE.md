# Developer Quick Reference Guide

## Adding a New Feature

### 1. Create Feature Structure
```bash
mkdir -p src/features/myfeature/{stores,services,repositories,handlers,components}
touch src/features/myfeature/index.ts
```

### 2. Define Entity (if new domain)
```typescript
// src/core/entities/MyFeature.ts
export interface MyFeature {
  id: MyFeatureId
  name: string
  createdAt: Date
}

export type MyFeatureId = string & { __brand: 'MyFeatureId' }
```

### 3. Create Repository
```typescript
// src/features/myfeature/repositories/myFeatureRepository.ts
export const myFeatureRepository = {
  async findById(id: MyFeatureId): Promise<MyFeature | null> {
    const response = await api.get(`/api/myfeature/${id}`)
    return response.data
  },
  
  async create(data: MyFeatureDraft): Promise<MyFeature> {
    const response = await api.post('/api/myfeature', data)
    return response.data
  }
}
```

### 4. Create Service
```typescript
// src/features/myfeature/services/myFeatureService.ts
class MyFeatureService {
  private get store() {
    return useMyFeatureStore()
  }

  async loadFeatures() {
    this.store.setLoading(true)
    try {
      const features = await myFeatureRepository.findAll()
      this.store.setFeatures(features)
    } finally {
      this.store.setLoading(false)
    }
  }
}

export const myFeatureService = new MyFeatureService()
```

### 5. Create Store
```typescript
// src/features/myfeature/stores/myFeatureStore.ts
export const useMyFeatureStore = defineStore('myFeature', () => {
  const features = ref<MyFeature[]>([])
  const loading = ref(false)
  
  const setFeatures = (items: MyFeature[]) => {
    features.value = items
  }
  
  return {
    features: readonly(features),
    loading: readonly(loading),
    setFeatures
  }
})
```

### 6. Create WebSocket Handler
```typescript
// src/features/myfeature/handlers/myFeatureSocketHandlers.ts
export function handleWebSocketEvent(event: WebSocketEvent) {
  switch (event.event) {
    case 'myfeature_created':
      handleCreated(event)
      break
  }
}

function handleCreated(event: WebSocketEvent) {
  const data = JSON.parse(event.data)
  myFeatureService.handleIncoming(data)
}
```

### 7. Register Handler
```typescript
// In a component setup or app initialization
import { useWebSocket } from '@/composables/useWebSocket'
import { handleWebSocketEvent } from '@/features/myfeature'

const { onEvent, offEvent } = useWebSocket()
onEvent('myfeature_created', handleWebSocketEvent)
```

---

## Common Patterns

### Loading Data
```typescript
// Component
const messages = computed(() => messageStore.getMessages(channelId))

onMounted(() => {
  messageService.loadMessages(channelId)
})
```

### Optimistic Updates
```typescript
// In Service
async sendMessage(draft: MessageDraft) {
  const optimistic = createOptimisticMessage(draft)
  
  // 1. Update UI immediately
  this.store.addMessage(optimistic)
  
  try {
    // 2. Make API call
    const real = await messageRepository.create(draft)
    // 3. Replace with real data
    this.store.replaceOptimistic(optimistic.id, real)
  } catch (error) {
    // 4. Mark as failed
    this.store.markFailed(optimistic.id)
    throw error
  }
}
```

### Error Handling
```typescript
// In Service
try {
  await messageRepository.create(draft)
} catch (error) {
  if (error instanceof AppError) {
    this.store.setError(error.message)
  } else {
    this.store.setError('Unknown error')
  }
  throw error
}

// In Component
async function send() {
  try {
    await messageService.sendMessage(draft)
  } catch (err) {
    toast.error('Failed to send message')
  }
}
```

---

## Do's and Don'ts

### ✅ DO
- Keep stores pure (state + simple mutations only)
- Put business logic in services
- Use repositories for all API calls
- Handle WebSocket events in feature handlers
- Use branded types for IDs (`MessageId`, `ChannelId`)
- Return Result types from repositories

### ❌ DON'T
- Call API directly from stores
- Put business logic in components
- Mix concerns in a single file
- Skip layers (Component → Service is wrong)
- Use `any` types for domain data
- Mutate state outside stores

---

## Testing

### Unit Test Service
```typescript
import { describe, it, expect, vi } from 'vitest'
import { MessageService } from './messageService'

describe('MessageService', () => {
  it('should optimistically add message', async () => {
    const mockRepo = {
      create: vi.fn().mockResolvedValue({ id: '123', content: 'Hello' })
    }
    const mockStore = {
      addMessage: vi.fn(),
      replaceOptimistic: vi.fn()
    }
    
    const service = new MessageService(mockRepo, mockStore)
    await service.sendMessage({ channelId: 'c1', content: 'Hello' })
    
    expect(mockStore.addMessage).toHaveBeenCalled()
    expect(mockRepo.create).toHaveBeenCalled()
  })
})
```

### Unit Test Store
```typescript
describe('useMessageStore', () => {
  it('should add message', () => {
    const store = useMessageStore()
    const message = createTestMessage()
    
    store.addMessage('channel1', message)
    
    expect(store.getMessages('channel1')).toContain(message)
  })
})
```

---

## Migration from Old Stores

### Before (Old Pattern)
```typescript
// stores/messages.ts (601 lines — legacy store, since deleted; see MIGRATION_GUIDE.md)
export const useMessageStore = defineStore('messages', {
  state: () => ({ messages: [] }),
  actions: {
    async loadMessages(channelId) {
      const response = await api.get(`/channels/${channelId}/posts`)
      this.messages = response.data
    },
    async sendMessage(content) {
      // 100+ lines of mixed logic
    }
  }
})
```

### After (New Pattern)
```typescript
// src/features/messages/stores/messageStore.ts
export const useMessageStore = defineStore('messageStore', () => {
  const messagesByChannel = ref(new Map())
  const setMessages = (id, msgs) => messagesByChannel.value.set(id, msgs)
  return { messagesByChannel, setMessages }
})

// src/features/messages/services/messageService.ts
export class MessageService {
  async loadMessages(channelId) {
    const { messages } = await messageRepository.findByChannel(channelId)
    this.store.setMessages(channelId, messages)
  }
}
```
