// Calls Feature - Public API
// Usage: import { useCallStore, callService, CallPanel } from '@/features/calls'

// Stores
export { useCallStore } from './stores/callStore'

// Services
export { callService } from './services/callService'

// Repositories
export { callRepository } from './repositories/callRepository'

// Handlers
export { handleCallWebSocketEvent } from './handlers/callSocketHandlers'

// Types
export type {
  CallState,
  CallConfig,
  CallParticipant,
  CurrentCallSession,
  IncomingCall,
  CallId,
  SessionId
} from '../../core/entities/Call'

// Components (to be created)
// export { default as CallPanel } from './components/CallPanel.vue'
// export { default as CallControls } from './components/CallControls.vue'
// export { default as IncomingCallModal } from './components/IncomingCallModal.vue'
