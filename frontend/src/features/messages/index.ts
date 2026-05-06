// Messages Feature - Public API
// Usage: import { useMessageStore, messageService, MessageList } from '@/features/messages'

// Stores
export { useMessageStore } from './stores/messageStore'
export { useThreadStore, type ThreadState } from './stores/threadStore'

// Services
export { messageService } from './services/messageService'
export { threadService, type ThreadResponse, type ThreadQueryParams } from './services/threadService'

// Repositories
export { messageRepository } from './repositories/messageRepository'

// Handlers
export { handleWebSocketEvent as handleMessageWebSocketEvent } from './handlers/messageSocketHandlers'
export { registerThreadHandlers } from './handlers/threadSocketHandlers'

// Composables (to be created)
// export { useMessages } from './composables/useMessages'
// export { useThread } from './composables/useThread'

// Components
// export { default as MessageList } from './components/MessageList.vue'
// export { default as MessageInput } from './components/MessageInput.vue'
export { default as ThreadPanel } from '../../components/thread/ThreadPanel.vue'
export { default as ThreadHeader } from '../../components/thread/ThreadHeader.vue'
export { default as ThreadReplyList } from '../../components/thread/ThreadReplyList.vue'
export { default as ThreadReplyItem } from '../../components/thread/ThreadReplyItem.vue'
export { default as ThreadComposer } from '../../components/composer/ThreadComposer.vue'
