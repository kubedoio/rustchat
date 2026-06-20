// Core Module - Public API
// Low-level primitives used across all features

// Entities
export type { User, UserId, UserRef, Presence } from './entities/User'
export type { Channel, ChannelId, ChannelType } from './entities/Channel'
export type {
  Message,
  MessageId,
  MessageDraft,
  MessageStatus,
  Reaction,
  FileAttachment,
} from './entities/Message'
export type { CallId, CallState, CallParticipant } from './entities/Call'

// Types
export type { EntityId } from './entities/Entity'
export { createEntityId } from './entities/Entity'

// Errors
export { AppError } from './errors/AppError'

// WebSocket
export { wsManager, useWebSocket, type ConnectionState } from './websocket/WebSocketManager'
