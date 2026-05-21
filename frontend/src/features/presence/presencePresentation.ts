import { Check, Circle, Clock3, Minus } from 'lucide-vue-next'
import type { Component } from 'vue'
import type { PresenceStatus } from '../../core/entities/User'

export interface PresencePresentation {
  status: PresenceStatus
  label: string
  icon: Component | null
  badgeClass: string
}

export function normalizePresenceStatus(value?: string | null): PresenceStatus {
  const normalized = value?.toLowerCase()
  if (
    normalized === 'online' ||
    normalized === 'away' ||
    normalized === 'dnd' ||
    normalized === 'offline'
  ) {
    return normalized
  }
  return 'offline'
}

export function getPresencePresentation(value?: string | null): PresencePresentation {
  const status = normalizePresenceStatus(value)

  switch (status) {
    case 'online':
      return {
        status,
        label: 'Online',
        icon: Check,
        badgeClass: 'bg-success text-white border-bg-surface-1',
      }
    case 'away':
      return {
        status,
        label: 'Away',
        icon: Clock3,
        badgeClass: 'bg-warning/15 text-warning border-bg-surface-1',
      }
    case 'dnd':
      return {
        status,
        label: 'Do not disturb',
        icon: Minus,
        badgeClass: 'bg-danger text-white border-bg-surface-1',
      }
    default:
      return {
        status,
        label: 'Offline',
        icon: Circle,
        badgeClass: 'bg-bg-surface-1 text-text-4 border-border-2',
      }
  }
}
