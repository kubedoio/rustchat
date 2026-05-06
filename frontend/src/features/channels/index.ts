// Channels Feature - Public API
// Usage: import { useChannelStore, channelService, ChannelList } from '@/features/channels'

// Stores
export { useChannelStore } from './stores/channelStore'

// Services
export { channelService } from './services/channelService'

// Repositories
export { channelRepository } from './repositories/channelRepository'

// Handlers
export { handleChannelWebSocketEvent } from './handlers/channelSocketHandlers'

// Types
export type {
  CreateChannelRequest,
  ChannelUnreadCounts
} from './repositories/channelRepository'

// Components (Svelte versions exist in src/svelte/components/)
