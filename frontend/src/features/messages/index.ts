// Messages Feature - Public API
// Note: Vue components have been migrated to Svelte. See src/svelte/components/

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
