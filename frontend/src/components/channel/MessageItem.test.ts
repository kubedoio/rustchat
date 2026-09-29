// @vitest-environment jsdom

import { reactive } from 'vue'
import type { Component } from 'vue'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { format } from 'date-fns'
import type { Message } from '@/features/messages/stores/messageStore'

// --- Module-level store mocks (paths match MessageItem.vue's real imports) ---

const authStore = reactive({
  user: { id: 'u1', username: 'alice' } as { id: string; username: string } | null,
})

const messageStore = {
  pinMessage: vi.fn<(_: string, __: string) => Promise<void>>(),
  unpinMessage: vi.fn<(_: string, __: string) => Promise<void>>(),
  saveMessage: vi.fn<(_: string, __: string) => Promise<void>>(),
  unsaveMessage: vi.fn<(_: string, __: string) => Promise<void>>(),
  handleMessageUpdate: vi.fn<(_: Record<string, unknown>) => void>(),
  handleReactionAdded: vi.fn<(_: Record<string, unknown>) => void>(),
  handleReactionRemoved: vi.fn<(_: Record<string, unknown>) => void>(),
}

const unreadStore = {
  markAsUnreadFromPost: vi.fn<(_: string) => Promise<void>>(),
}

const uiStore = reactive({
  density: 'comfortable' as string,
  openVideoCall: vi.fn<(_: string) => void>(),
})

const configStore = reactive({
  siteConfig: { post_edit_time_limit_seconds: -1 as number | null },
})

const postsApi = {
  delete: vi.fn<(_: string) => Promise<void>>(),
  update: vi.fn<(_: string, __: string) => Promise<{ data: Record<string, unknown> }>>(),
  addReaction: vi.fn<(_: string, __: string) => Promise<void>>(),
  removeReaction: vi.fn<(_: string, __: string) => Promise<void>>(),
}

// agentsApi is mocked even though no test exercises the bot-feedback branch:
// the component auto-calls agentsApi.getFeedbackSummary on mount for bot
// messages, and leaving it real would silently issue HTTP requests the day
// someone adds an isBot fixture.
const agentsApi = {
  getFeedbackSummary: vi.fn<(_: string) => Promise<Record<string, unknown>>>(),
  submitFeedback: vi.fn().mockResolvedValue(undefined),
  deleteFeedback: vi.fn().mockResolvedValue(undefined),
}

// window.confirm / window.open are stubbed for the whole file
const confirmMock = vi.fn(() => true)
const openMock = vi.fn(() => null)

vi.mock('@/features/messages/stores/messageStore', () => ({ useMessageStore: () => messageStore }))
vi.mock('../../features/auth/stores/authStore', () => ({ useAuthStore: () => authStore }))
vi.mock('@/features/unreads/stores/unreadStore', () => ({ useUnreadStore: () => unreadStore }))
vi.mock('../../features/ui/stores/uiStore', () => ({ useUIStore: () => uiStore }))
vi.mock('../../features/config/stores/configStore', () => ({ useConfigStore: () => configStore }))
vi.mock('../../api/posts', () => ({ postsApi }))
vi.mock('../../api/agents', () => ({ agentsApi }))
// Identity markdown renderer: keeps v-html content assertions deterministic and
// avoids pulling in marked/highlight.js during these tests.
vi.mock('../../composables/useMarkdownRenderer', () => ({
  useMarkdownRenderer: () => ({
    renderMarkdown: (content: string) => content,
  }),
}))

// --- Stubbed children ---

// Mimics FilePreview's public API: renders the file name and re-emits preview on click
const FilePreviewStub: Component = {
  name: 'FilePreview',
  props: { file: { type: Object, required: true } },
  emits: ['preview'],
  template: `<div data-testid="file-preview" @click="$emit('preview', file)">{{ file.name }}</div>`,
}

// Renders the gallery image count so tests can assert the image-only filter
const ImageGalleryStub: Component = {
  name: 'ImageGallery',
  props: {
    images: { type: Array, required: true },
    initialIndex: { type: Number, required: true },
  },
  template: `<div data-testid="image-gallery">{{ images.length }}</div>`,
}

// --- Fixtures and helpers ---

const FIXED_TS = '2026-01-15T10:30:00.000Z'

// Default fixture is a message from another user ("bob"); pass userId: 'u1'
// (the mocked auth user "alice") for own-message scenarios.
function makeMessage(overrides: Partial<Message> = {}): Message {
  return {
    id: 'm1',
    channelId: 'ch1',
    userId: 'u2',
    username: 'bob',
    content: 'Hello world',
    timestamp: FIXED_TS,
    reactions: [],
    isPinned: false,
    isSaved: false,
    seq: 1,
    ...overrides,
  }
}

async function mountMessage(overrides: Partial<Message> = {}) {
  const MessageItem = (await import('./MessageItem.vue')).default
  return mount(MessageItem, {
    props: { message: makeMessage(overrides) },
    global: {
      stubs: {
        RcAvatar: true,
        EmojiPicker: true,
        FilePreview: FilePreviewStub,
        ImageGallery: ImageGalleryStub,
        teleport: true,
      },
    },
  })
}

async function openMenu(wrapper: VueWrapper) {
  // The component has a fragment root (template comments + v-if branches), so
  // wrapper.element is VTU's app container; the real root is its first element child.
  ;(wrapper.element.firstElementChild as HTMLElement).dispatchEvent(new MouseEvent('mouseenter'))
  await wrapper.find('button[title="More actions"]').trigger('click')
}

// Classes of the actual message root element (see openMenu for why wrapper.classes()
// cannot be used directly)
function rootClasses(wrapper: VueWrapper) {
  return Array.from((wrapper.element.firstElementChild as HTMLElement).classList)
}

function findButton(wrapper: VueWrapper, label: string) {
  return wrapper.findAll('button').find(b => b.text() === label)
}

function findButtonByTitle(wrapper: VueWrapper, title: string) {
  // Attribute selectors cannot contain emoji (nwsapi), so match titles in JS
  return wrapper.findAll('button').find(b => b.attributes('title') === title)
}

const imageFile = {
  id: 'f1',
  name: 'cat.png',
  url: 'https://example.com/cat.png',
  size: 1024,
  mime_type: 'image/png',
}
const pdfFile = {
  id: 'f2',
  name: 'notes.pdf',
  url: 'https://example.com/notes.pdf',
  size: 2048,
  mime_type: 'application/pdf',
}

describe('MessageItem', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    authStore.user = { id: 'u1', username: 'alice' }
    uiStore.density = 'comfortable'
    configStore.siteConfig.post_edit_time_limit_seconds = -1
    confirmMock.mockReturnValue(true)
    vi.stubGlobal('confirm', confirmMock)
    vi.stubGlobal('open', openMock)
    postsApi.delete.mockResolvedValue(undefined)
    postsApi.update.mockResolvedValue({
      data: { id: 'm1', message: 'Updated content', edit_at: FIXED_TS },
    })
    postsApi.addReaction.mockResolvedValue(undefined)
    postsApi.removeReaction.mockResolvedValue(undefined)
    messageStore.pinMessage.mockResolvedValue(undefined)
    messageStore.unpinMessage.mockResolvedValue(undefined)
    messageStore.saveMessage.mockResolvedValue(undefined)
    messageStore.unsaveMessage.mockResolvedValue(undefined)
    unreadStore.markAsUnreadFromPost.mockResolvedValue(undefined)
  })

  afterEach(() => {
    // Safety net for tests that fake the clock
    vi.useRealTimers()
    // Restore confirm/open so the stubs cannot leak into other test files
    // that share this vitest worker.
    vi.unstubAllGlobals()
  })

  // --- 1. Regular message rendering ---

  it('renders username, content and formatted timestamp', async () => {
    const wrapper = await mountMessage()
    expect(wrapper.text()).toContain('bob')
    expect(wrapper.text()).toContain('Hello world')
    expect(wrapper.text()).toContain(format(new Date(FIXED_TS), 'h:mm a'))
  })

  it('marks outgoing messages as sending', async () => {
    const wrapper = await mountMessage({ status: 'sending' })
    expect(wrapper.text()).toContain('Sending...')
    expect(rootClasses(wrapper)).toContain('opacity-70')
  })

  it('marks failed messages', async () => {
    const wrapper = await mountMessage({ status: 'failed' })
    expect(wrapper.text()).toContain('Failed')
    expect(rootClasses(wrapper)).toContain('bg-danger/5')
  })

  it('renders pinned/saved badges and the (edited) marker', async () => {
    const wrapper = await mountMessage({
      isPinned: true,
      isSaved: true,
      editedAt: '2026-01-15T10:31:00.000Z',
    })
    expect(wrapper.text()).toContain('Pinned')
    expect(wrapper.text()).toContain('Saved')
    expect(wrapper.text()).toContain('(edited)')
  })

  it('emits openProfile when the avatar is clicked', async () => {
    const wrapper = await mountMessage()
    await wrapper.find('[data-testid="message-avatar"]').trigger('click')
    expect(wrapper.emitted('openProfile')).toEqual([['u2']])
  })

  it('shows edit and delete actions for own messages', async () => {
    const wrapper = await mountMessage({ userId: 'u1', username: 'alice' })
    await openMenu(wrapper)
    expect(findButton(wrapper, 'Edit message')).toBeDefined()
    expect(findButton(wrapper, 'Delete message')).toBeDefined()
  })

  it('hides edit and delete actions for other users messages', async () => {
    const wrapper = await mountMessage()
    await openMenu(wrapper)
    expect(findButton(wrapper, 'Edit message')).toBeUndefined()
    expect(findButton(wrapper, 'Delete message')).toBeUndefined()
    // common actions remain available to everyone
    expect(findButton(wrapper, 'Save message')).toBeDefined()
    expect(findButton(wrapper, 'Pin to channel')).toBeDefined()
  })

  // --- 2. System message branch ---

  it('renders system messages as one-liners without avatar or actions', async () => {
    const wrapper = await mountMessage({
      props: { type: 'system_join_leave' },
      content: 'bob joined the channel',
    })
    expect(wrapper.find('[data-testid="message-avatar"]').exists()).toBe(false)
    expect(wrapper.findAll('button').length).toBe(0)
    expect(wrapper.find('div.italic').exists()).toBe(true)
    expect(wrapper.text()).toContain('bob joined the channel')
    expect(wrapper.text()).toContain(format(new Date(FIXED_TS), 'h:mm a'))
  })

  // --- 3. Video call messages ---

  it('shows a join button for an ongoing call and opens it in a new window', async () => {
    const wrapper = await mountMessage({
      props: { type: 'video_call', meeting_url: 'https://meet.example.com/room' },
    })
    expect(wrapper.text()).toContain('Video Call')
    expect(wrapper.text()).toContain('Ongoing call')
    await findButton(wrapper, 'Join Call')!.trigger('click')
    expect(openMock).toHaveBeenCalledWith(
      'https://meet.example.com/room',
      '_blank',
      'noopener,noreferrer'
    )
  })

  it('hides the join button once the call has ended', async () => {
    const wrapper = await mountMessage({
      props: {
        type: 'video_call',
        status: 'ended',
        duration_text: '5m 12s',
        meeting_url: 'https://meet.example.com/room',
      },
    })
    expect(findButton(wrapper, 'Join Call')).toBeUndefined()
    expect(wrapper.text()).toContain('Call ended')
    expect(wrapper.text()).toContain('5m 12s')
  })

  it('opens embedded calls through the UI store instead of a new window', async () => {
    const wrapper = await mountMessage({
      props: {
        type: 'video_call',
        meeting_url: 'https://meet.example.com/room',
        mode: 'embed_iframe',
      },
    })
    await findButton(wrapper, 'Join Call')!.trigger('click')
    expect(uiStore.openVideoCall).toHaveBeenCalledWith('https://meet.example.com/room')
    expect(openMock).not.toHaveBeenCalled()
  })

  it('renders calls-protocol messages as started/ended one-liners', async () => {
    const ongoing = await mountMessage({
      props: { type: 'custom_com.mattermost.calls' },
    })
    expect(ongoing.text()).toContain('started a call')
    // No join affordance on the legacy calls-protocol rendering
    expect(findButton(ongoing, 'Join Call')).toBeUndefined()

    const ended = await mountMessage({
      props: { type: 'custom_com.mattermost.calls', end_at: FIXED_TS, duration: 125000 },
    })
    expect(ended.text()).toContain('ended the call')
    expect(ended.text()).toContain('2m 5s')
  })

  // --- 4. Edit flow ---

  it('enters edit mode with the content prefilled and cancels on Escape', async () => {
    const wrapper = await mountMessage({
      userId: 'u1',
      username: 'alice',
      content: 'Original content',
    })
    await openMenu(wrapper)
    await findButton(wrapper, 'Edit message')!.trigger('click')

    const textarea = wrapper.find('textarea')
    expect(textarea.exists()).toBe(true)
    expect((textarea.element as HTMLTextAreaElement).value).toBe('Original content')

    await textarea.trigger('keydown', { key: 'Escape' })
    expect(wrapper.find('textarea').exists()).toBe(false)
    expect(postsApi.update).not.toHaveBeenCalled()
    expect(wrapper.emitted('update')).toBeUndefined()
  })

  it('saves edits on Enter, updates the store and emits update', async () => {
    const wrapper = await mountMessage({
      userId: 'u1',
      username: 'alice',
      content: 'Original content',
    })
    await openMenu(wrapper)
    await findButton(wrapper, 'Edit message')!.trigger('click')

    await wrapper.find('textarea').setValue('Updated content')
    await wrapper.find('textarea').trigger('keydown', { key: 'Enter' })
    await flushPromises()

    expect(postsApi.update).toHaveBeenCalledWith('m1', 'Updated content')
    expect(messageStore.handleMessageUpdate).toHaveBeenCalledTimes(1)
    expect(messageStore.handleMessageUpdate).toHaveBeenCalledWith({
      id: 'm1',
      message: 'Updated content',
      edit_at: FIXED_TS,
    })
    expect(wrapper.emitted('update')).toEqual([['m1', 'Updated content']])
    expect(wrapper.find('textarea').exists()).toBe(false)
  })

  it('treats unchanged content as a no-op when saving', async () => {
    const wrapper = await mountMessage({
      userId: 'u1',
      username: 'alice',
      content: 'Original content',
    })
    await openMenu(wrapper)
    await findButton(wrapper, 'Edit message')!.trigger('click')
    await findButton(wrapper, 'Save')!.trigger('click')

    expect(postsApi.update).not.toHaveBeenCalled()
    expect(wrapper.emitted('update')).toBeUndefined()
    expect(wrapper.find('textarea').exists()).toBe(false)
  })

  it('treats empty content as a no-op when saving', async () => {
    const wrapper = await mountMessage({
      userId: 'u1',
      username: 'alice',
      content: 'Original content',
    })
    await openMenu(wrapper)
    await findButton(wrapper, 'Edit message')!.trigger('click')

    await wrapper.find('textarea').setValue('   ')
    await wrapper.find('textarea').trigger('keydown', { key: 'Enter' })

    expect(postsApi.update).not.toHaveBeenCalled()
    expect(wrapper.emitted('update')).toBeUndefined()
    expect(wrapper.find('textarea').exists()).toBe(false)
  })

  it('disables the save button while the update is in flight', async () => {
    let resolveUpdate!: (value: { data: Record<string, unknown> }) => void
    postsApi.update.mockImplementation(
      () =>
        new Promise<{ data: Record<string, unknown> }>(resolve => {
          resolveUpdate = resolve
        })
    )
    const wrapper = await mountMessage({
      userId: 'u1',
      username: 'alice',
      content: 'Original content',
    })
    await openMenu(wrapper)
    await findButton(wrapper, 'Edit message')!.trigger('click')
    await wrapper.find('textarea').setValue('Updated content')
    await wrapper.find('textarea').trigger('keydown', { key: 'Enter' })

    const saveButton = wrapper.findAll('button').find(b => b.text().includes('Sav'))
    expect(saveButton).toBeDefined()
    expect(saveButton!.attributes('disabled')).toBeDefined()
    expect(saveButton!.text()).toContain('Saving...')

    resolveUpdate({ data: { id: 'm1', message: 'Updated content', edit_at: FIXED_TS } })
    await flushPromises()
    expect(wrapper.emitted('update')).toEqual([['m1', 'Updated content']])
    expect(wrapper.find('textarea').exists()).toBe(false)
  })

  it('falls back to a synthesized edited_at when the update response omits it', async () => {
    // Legacy responses can omit both edited_at and edit_at; the component then
    // synthesizes the marker payload itself.
    postsApi.update.mockResolvedValue({ data: { id: 'm1', message: 'Updated content' } })
    const wrapper = await mountMessage({
      userId: 'u1',
      username: 'alice',
      content: 'Original content',
    })
    await openMenu(wrapper)
    await findButton(wrapper, 'Edit message')!.trigger('click')
    await wrapper.find('textarea').setValue('Updated content')
    await wrapper.find('textarea').trigger('keydown', { key: 'Enter' })
    await flushPromises()

    expect(messageStore.handleMessageUpdate).toHaveBeenCalledTimes(2)
    const secondCall = messageStore.handleMessageUpdate.mock.calls[1][0]
    expect(secondCall.id).toBe('m1')
    expect(secondCall.message).toBe('Updated content')
    expect(typeof secondCall.edited_at).toBe('string')
    expect(new Date(secondCall.edited_at).getTime()).not.toBeNaN()
    expect(wrapper.emitted('update')).toEqual([['m1', 'Updated content']])
  })

  // --- 5. Edit time limit ---

  it('hides the edit action when the message is older than the time limit', async () => {
    // Only fake Date: flushPromises relies on the real setTimeout
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-01-15T12:00:00Z'))
    configStore.siteConfig.post_edit_time_limit_seconds = 60
    try {
      // 61 seconds old — one second past the 60s window
      const wrapper = await mountMessage({
        userId: 'u1',
        username: 'alice',
        timestamp: '2026-01-15T11:58:59.000Z',
      })
      await openMenu(wrapper)
      expect(findButton(wrapper, 'Edit message')).toBeUndefined()
      // delete is not time-limited
      expect(findButton(wrapper, 'Delete message')).toBeDefined()
    } finally {
      vi.useRealTimers()
    }
  })

  it('always allows editing when the time limit is disabled (-1)', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-01-15T12:00:00Z'))
    configStore.siteConfig.post_edit_time_limit_seconds = -1
    try {
      // two hours old, but no limit is enforced
      const wrapper = await mountMessage({
        userId: 'u1',
        username: 'alice',
        timestamp: '2026-01-15T10:00:00.000Z',
      })
      await openMenu(wrapper)
      expect(findButton(wrapper, 'Edit message')).toBeDefined()
    } finally {
      vi.useRealTimers()
    }
  })

  it('never allows editing when the time limit is 0', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date('2026-01-15T12:00:00Z'))
    configStore.siteConfig.post_edit_time_limit_seconds = 0
    try {
      const wrapper = await mountMessage({ userId: 'u1', username: 'alice' })
      await openMenu(wrapper)
      expect(findButton(wrapper, 'Edit message')).toBeUndefined()
    } finally {
      vi.useRealTimers()
    }
  })

  // --- 6. Delete flow ---

  it('does not delete when the user cancels the confirmation dialog', async () => {
    confirmMock.mockReturnValue(false)
    const wrapper = await mountMessage({ userId: 'u1', username: 'alice' })
    await openMenu(wrapper)
    await findButton(wrapper, 'Delete message')!.trigger('click')

    expect(confirmMock).toHaveBeenCalledWith('Delete this message?')
    expect(postsApi.delete).not.toHaveBeenCalled()
    expect(wrapper.emitted('delete')).toBeUndefined()
  })

  it('deletes through the API and emits delete after confirmation', async () => {
    const wrapper = await mountMessage({ userId: 'u1', username: 'alice' })
    await openMenu(wrapper)
    await findButton(wrapper, 'Delete message')!.trigger('click')
    await flushPromises()

    expect(postsApi.delete).toHaveBeenCalledWith('m1')
    expect(wrapper.emitted('delete')).toEqual([['m1']])
  })

  // --- 7. Reactions ---

  it('renders reactions with counts and highlights the own reaction', async () => {
    const wrapper = await mountMessage({
      reactions: [
        // current user (u1) reacted to the first emoji only
        { emoji: ':+1:', apiKey: '+1', count: 3, users: ['u1', 'u2', 'u3'] },
        { emoji: ':heart:', apiKey: 'heart', count: 1, users: ['u2'] },
      ],
    })
    const buttons = wrapper.findAll('button.rounded-full')
    expect(buttons.length).toBe(2)
    expect(buttons[0].text()).toContain('👍')
    expect(buttons[0].text()).toContain('3')
    expect(buttons[0].classes()).toContain('bg-brand/10')
    expect(buttons[1].text()).toContain('1')
    expect(buttons[1].classes()).not.toContain('bg-brand/10')
  })

  it('removes the reaction when clicking an emoji the current user already reacted with', async () => {
    const wrapper = await mountMessage({
      reactions: [{ emoji: ':+1:', apiKey: '+1', count: 2, users: ['u1', 'u2'] }],
    })
    await wrapper.find('button.rounded-full').trigger('click')
    await flushPromises()

    expect(postsApi.removeReaction).toHaveBeenCalledWith('m1', '+1')
    expect(messageStore.handleReactionRemoved).toHaveBeenCalledWith({
      post_id: 'm1',
      user_id: 'u1',
      emoji_name: '+1',
    })
    expect(postsApi.addReaction).not.toHaveBeenCalled()
  })

  it('adds a reaction when clicking an emoji the current user has not reacted with', async () => {
    const wrapper = await mountMessage({
      reactions: [{ emoji: ':+1:', apiKey: '+1', count: 1, users: ['u2'] }],
    })
    await wrapper.find('button.rounded-full').trigger('click')
    await flushPromises()

    expect(postsApi.addReaction).toHaveBeenCalledWith('m1', '+1')
    expect(messageStore.handleReactionAdded).toHaveBeenCalledWith({
      post_id: 'm1',
      user_id: 'u1',
      emoji_name: '+1',
    })
    expect(postsApi.removeReaction).not.toHaveBeenCalled()
  })

  it('adds a reaction from the quick emoji bar', async () => {
    const wrapper = await mountMessage()
    ;(wrapper.element.firstElementChild as HTMLElement).dispatchEvent(new MouseEvent('mouseenter'))
    await findButtonByTitle(wrapper, 'React with 👍')!.trigger('click')
    await flushPromises()

    expect(postsApi.addReaction).toHaveBeenCalledWith('m1', '+1')
    expect(messageStore.handleReactionAdded).toHaveBeenCalledWith({
      post_id: 'm1',
      user_id: 'u1',
      emoji_name: '+1',
    })
  })

  // --- 8. Thread indicator ---

  it('shows the thread reply count and emits reply on click', async () => {
    const wrapper = await mountMessage({ threadCount: 2 })
    const threadButton = wrapper.findAll('button').find(b => b.text().includes('2 replies'))
    expect(threadButton).toBeDefined()
    await threadButton!.trigger('click')
    expect(wrapper.emitted('reply')).toEqual([['m1']])
  })

  it('pluralizes a single reply', async () => {
    const wrapper = await mountMessage({ threadCount: 1 })
    expect(wrapper.text()).toContain('1 reply')
  })

  it('hides the thread indicator when there are no replies', async () => {
    const wrapper = await mountMessage({ threadCount: 0 })
    expect(wrapper.text()).not.toContain('replies')
    expect(wrapper.text()).not.toContain('reply')
  })

  // --- 9. Mentions ---

  it('highlights the message container when the current user is mentioned', async () => {
    const wrapper = await mountMessage({ content: 'hey @alice check this out' })
    expect(rootClasses(wrapper)).toContain('bg-brand/5')
  })

  it('does not highlight for mentions of other users', async () => {
    const wrapper = await mountMessage({ content: 'hey @bob check this out' })
    expect(rootClasses(wrapper)).not.toContain('bg-brand/5')
  })

  // --- 10. Files ---

  it('renders a preview for each attached file', async () => {
    const wrapper = await mountMessage({ files: [imageFile, pdfFile] })
    const previews = wrapper.findAll('[data-testid="file-preview"]')
    expect(previews.length).toBe(2)
    expect(previews[0].text()).toContain('cat.png')
    expect(previews[1].text()).toContain('notes.pdf')
    expect(wrapper.find('[data-testid="image-gallery"]').exists()).toBe(false)
  })

  it('opens the gallery with only image files when an image is previewed', async () => {
    const wrapper = await mountMessage({ files: [imageFile, pdfFile] })
    await wrapper.findAll('[data-testid="file-preview"]')[0].trigger('click')
    const gallery = wrapper.find('[data-testid="image-gallery"]')
    expect(gallery.exists()).toBe(true)
    expect(gallery.text()).toBe('1')
  })

  // --- Menu store actions ---

  it('pins through the message store and closes the menu', async () => {
    const wrapper = await mountMessage()
    await openMenu(wrapper)
    await findButton(wrapper, 'Pin to channel')!.trigger('click')
    await flushPromises()

    expect(messageStore.pinMessage).toHaveBeenCalledWith('m1', 'ch1')
    expect(findButton(wrapper, 'Pin to channel')).toBeUndefined()
  })

  it('unpins an already pinned message', async () => {
    const wrapper = await mountMessage({ isPinned: true })
    await openMenu(wrapper)
    await findButton(wrapper, 'Unpin from channel')!.trigger('click')
    await flushPromises()

    expect(messageStore.unpinMessage).toHaveBeenCalledWith('m1', 'ch1')
  })

  it('saves through the message store', async () => {
    const wrapper = await mountMessage()
    await openMenu(wrapper)
    await findButton(wrapper, 'Save message')!.trigger('click')
    await flushPromises()

    expect(messageStore.saveMessage).toHaveBeenCalledWith('m1', 'ch1')
  })

  it('marks the channel as unread from this message', async () => {
    const wrapper = await mountMessage()
    await openMenu(wrapper)
    await findButton(wrapper, 'Mark as unread')!.trigger('click')
    await flushPromises()

    expect(unreadStore.markAsUnreadFromPost).toHaveBeenCalledWith('m1')
  })
})
