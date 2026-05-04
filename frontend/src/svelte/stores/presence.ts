import { writable } from 'svelte/store'

export interface TypingUser {
  userId: string
  username: string
  channelId: string
  threadRootId?: string
  timestamp?: number
}

export const typingUsers = writable<Map<string, TypingUser>>(new Map())

export function addTypingUser(key: string, user: TypingUser) {
  typingUsers.update((m) => {
    m.set(key, user)
    return m
  })
}

export function removeTypingUser(key: string) {
  typingUsers.update((m) => {
    m.delete(key)
    return m
  })
}
