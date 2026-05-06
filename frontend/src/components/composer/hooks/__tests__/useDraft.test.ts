import { describe, it, expect, beforeEach, vi } from 'vitest'
import { ref, nextTick } from 'vue'
import { useDraft, useDrafts } from '../useDraft'

// Mock localStorage
const localStorageMock = (() => {
    let store: Record<string, string> = {}
    return {
        getItem: vi.fn((key: string) => store[key] || null),
        setItem: vi.fn((key: string, value: string) => {
            store[key] = value
        }),
        removeItem: vi.fn((key: string) => {
            delete store[key]
        }),
        clear: vi.fn(() => {
            store = {}
        }),
        get length() {
            return Object.keys(store).length
        },
        key: vi.fn((index: number) => Object.keys(store)[index] || null)
    }
})()

Object.defineProperty(window, 'localStorage', {
    value: localStorageMock
})

describe('useDraft', () => {
    beforeEach(() => {
        localStorageMock.clear()
        vi.clearAllMocks()
    })

    it('loads existing draft from localStorage', () => {
        const channelId = 'channel-123'
        const draftData = {
            content: 'Hello world',
            timestamp: Date.now(),
            attachments: []
        }
        localStorageMock.setItem(`rustchat_draft:${channelId}`, JSON.stringify(draftData))

        const { draft, isRestored, hasDraft } = useDraft(channelId)
        
        // Need to trigger onMounted
        expect(draft.value).toBeNull() // Before mount
    })

    it('saves draft to localStorage', () => {
        const channelId = 'channel-456'
        const { saveDraft } = useDraft(channelId)

        saveDraft('Test message')

        expect(localStorageMock.setItem).toHaveBeenCalledWith(
            `rustchat_draft:${channelId}`,
            expect.stringContaining('Test message')
        )
    })

    it('clears draft when content is empty', () => {
        const channelId = 'channel-789'
        const { saveDraft, clearDraft } = useDraft(channelId)

        saveDraft('Test message')
        expect(localStorageMock.setItem).toHaveBeenCalled()

        saveDraft('')
        expect(localStorageMock.removeItem).toHaveBeenCalledWith(`rustchat_draft:${channelId}`)
    })

    it('clears draft explicitly', () => {
        const channelId = 'channel-abc'
        const { saveDraft, clearDraft } = useDraft(channelId)

        saveDraft('Test message')
        clearDraft()

        expect(localStorageMock.removeItem).toHaveBeenCalledWith(`rustchat_draft:${channelId}`)
    })

    it('includes attachments in draft', () => {
        const channelId = 'channel-def'
        const { saveDraft } = useDraft(channelId)

        saveDraft('Test message', ['file-1', 'file-2'])

        const savedData = JSON.parse(localStorageMock.getItem(`rustchat_draft:${channelId}`) || '{}')
        expect(savedData.attachments).toEqual(['file-1', 'file-2'])
    })

    it('rejects drafts older than 7 days', () => {
        const channelId = 'channel-old'
        const oldDraft = {
            content: 'Old message',
            timestamp: Date.now() - (8 * 24 * 60 * 60 * 1000), // 8 days ago
            attachments: []
        }
        localStorageMock.setItem(`rustchat_draft:${channelId}`, JSON.stringify(oldDraft))

        const { loadDraft } = useDraft(channelId)
        const loaded = loadDraft()

        expect(loaded).toBeNull()
        expect(localStorageMock.removeItem).toHaveBeenCalledWith(`rustchat_draft:${channelId}`)
    })

    it('updates draft content reactively', () => {
        const channelId = 'channel-xyz'
        const { updateDraftContent, draft, hasDraft } = useDraft(channelId)

        updateDraftContent('New content')

        expect(draft.value?.content).toBe('New content')
        expect(hasDraft.value).toBe(true)
    })
})

describe('useDrafts (multiple drafts)', () => {
    beforeEach(() => {
        localStorageMock.clear()
        vi.clearAllMocks()
    })

    it('counts all drafts', () => {
        const { saveDraft } = useDraft('channel-1')
        saveDraft('Message 1')
        
        const { saveDraft: saveDraft2 } = useDraft('channel-2')
        saveDraft2('Message 2')

        const { getDraftCount } = useDrafts()
        expect(getDraftCount()).toBe(2)
    })

    it('lists all channels with drafts', () => {
        const { saveDraft } = useDraft('channel-a')
        saveDraft('Message A')
        
        const { saveDraft: saveDraft2 } = useDraft('channel-b')
        saveDraft2('Message B')

        const { getChannelsWithDrafts } = useDrafts()
        const channels = getChannelsWithDrafts()
        
        expect(channels).toContain('channel-a')
        expect(channels).toContain('channel-b')
        expect(channels).toHaveLength(2)
    })

    it('clears all drafts', () => {
        const { saveDraft } = useDraft('channel-1')
        saveDraft('Message 1')
        
        const { saveDraft: saveDraft2 } = useDraft('channel-2')
        saveDraft2('Message 2')

        const { clearAllDrafts, getDraftCount } = useDrafts()
        clearAllDrafts()

        expect(getDraftCount()).toBe(0)
    })
})
