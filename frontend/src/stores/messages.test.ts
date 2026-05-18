import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useMessageStore } from '@/features/messages/stores/messageStore'
import type { Message } from '@/features/messages/stores/messageStore'

function buildMessage(overrides: Partial<Message> = {}): Message {
  return {
    id: overrides.id || 'post-1',
    channelId: overrides.channelId || 'channel-1',
    userId: overrides.userId || 'user-1',
    username: overrides.username || 'alice',
    content: overrides.content || 'hello',
    timestamp: overrides.timestamp || new Date().toISOString(),
    reactions: overrides.reactions || [],
    isPinned: overrides.isPinned ?? false,
    isSaved: overrides.isSaved ?? false,
    seq: overrides.seq ?? 0,
    ...overrides,
  }
}

describe('message reaction normalization', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('merges websocket alias reactions into the same top-level reaction bucket', () => {
    const store = useMessageStore()
    store.messagesByChannel['channel-1'] = [
      buildMessage({
        reactions: [
          {
            emoji: '👍',
            apiKey: '+1',
            count: 1,
            users: ['user-a'],
          },
        ],
      }),
    ]

    store.handleReactionAdded({
      post_id: 'post-1',
      user_id: 'user-b',
      emoji_name: 'thumbsup',
    })

    const reactions = store.messagesByChannel['channel-1']?.[0]?.reactions || []
    expect(reactions).toHaveLength(1)
    expect(reactions[0]?.emoji).toBe('👍')
    expect(reactions[0]?.users).toEqual(['user-a', 'user-b'])
    expect(reactions[0]?.count).toBe(2)
  })

  it('removes alias reactions from loaded thread replies without leaving stale buckets', () => {
    const store = useMessageStore()
    store.repliesByThread['root-1'] = [
      buildMessage({
        id: 'reply-1',
        rootId: 'root-1',
        reactions: [
          {
            emoji: '👍',
            apiKey: '+1',
            count: 1,
            users: ['user-a'],
          },
        ],
      }),
    ]

    store.handleReactionRemoved({
      post_id: 'reply-1',
      user_id: 'user-a',
      emoji_name: ':+1:',
    })

    const reactions = store.repliesByThread['root-1']?.[0]?.reactions || []
    expect(reactions).toEqual([])
  })
})
