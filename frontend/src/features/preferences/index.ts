// Preferences Feature - Public API
export { usePreferencesStore } from './stores/preferencesStore'
export { preferencesService } from './services/preferencesService'
export { preferencesRepository } from './repositories/preferencesRepository'
export type {
  UserStatus,
  UserPreferences,
  StatusPreset,
} from './repositories/preferencesRepository'
