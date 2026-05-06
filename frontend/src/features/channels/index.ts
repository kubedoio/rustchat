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

// Components (to be created)
// export { default as ChannelList } from './components/ChannelList.vue'
// export { default as ChannelSidebar } from './components/ChannelSidebar.vue'
// export { default as CreateChannelModal } from './components/CreateChannelModal.vue'
