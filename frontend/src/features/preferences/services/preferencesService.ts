// Preferences Service - Business logic for user preferences

import { log } from '@/utils/log'
import {
  preferencesRepository,
  type UserPreferences,
  type StatusPreset,
} from '../repositories/preferencesRepository'
import { usePreferencesStore } from '../stores/preferencesStore'
import { AppError } from '../../../core/errors/AppError'

class PreferencesService {
  private get store() {
    return usePreferencesStore()
  }

  // Load user status
  async loadStatus(): Promise<void> {
    try {
      const status = await preferencesRepository.getMyStatus()
      this.store.setStatus(status)
    } catch (error) {
      log.error('Failed to load status:', error)
    }
  }

  // Update user status
  async updateStatus(data: {
    text?: string
    emoji?: string
    durationMinutes?: number
  }): Promise<void> {
    this.store.setLoading(true)
    try {
      const status = await preferencesRepository.updateMyStatus(data)
      this.store.setStatus(status)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to update status')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Clear user status
  async clearStatus(): Promise<void> {
    this.store.setLoading(true)
    try {
      const status = await preferencesRepository.clearMyStatus()
      this.store.setStatus(status)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to clear status')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Load user preferences
  async loadPreferences(): Promise<void> {
    try {
      const prefs = await preferencesRepository.getMyPreferences()
      this.store.setPreferences(prefs)
    } catch (error) {
      log.error('Failed to load preferences:', error)
    }
  }

  // Update user preferences
  async updatePreferences(data: UserPreferences): Promise<void> {
    this.store.setLoading(true)
    try {
      const prefs = await preferencesRepository.updateMyPreferences(data)
      this.store.setPreferences(prefs)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to update preferences'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Load status presets
  async loadStatusPresets(): Promise<void> {
    try {
      const presets = await preferencesRepository.listStatusPresets()
      this.store.setStatusPresets(presets)
    } catch (error) {
      log.error('Failed to load status presets:', error)
    }
  }

  // Apply a status preset
  async applyPreset(preset: StatusPreset): Promise<void> {
    await this.updateStatus({
      text: preset.text,
      emoji: preset.emoji,
      durationMinutes: preset.durationMinutes,
    })
  }

  // Toggle sidebar category collapsed state
  toggleCollapsedCategory(categoryId: string): void {
    const prefs = this.store.preferences
    if (!prefs) return

    const collapsed = new Set(prefs.collapsedCategories || [])
    if (collapsed.has(categoryId)) {
      collapsed.delete(categoryId)
    } else {
      collapsed.add(categoryId)
    }

    void this.updatePreferences({
      ...prefs,
      collapsedCategories: Array.from(collapsed),
    })
  }
}

export const preferencesService = new PreferencesService()
