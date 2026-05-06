import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { ref } from 'vue'
import MattermostComposer from '../MattermostComposer.vue'

// Mock dependencies
vi.mock('../../../composables/useWebSocket', () => ({
    useWebSocket: () => ({
        sendTyping: vi.fn()
    })
}))

vi.mock('../../../stores/teams', () => ({
    useTeamStore: () => ({
        members: []
    })
}))

vi.mock('../../../stores/channels', () => ({
    useChannelStore: () => ({
        channels: []
    })
}))

vi.mock('../../../api/files', () => ({
    filesApi: {
        upload: vi.fn().mockResolvedValue({ data: { id: 'file-123', name: 'test.txt' } })
    }
}))

describe('MattermostComposer Integration', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('renders with correct placeholder', () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        const textarea = wrapper.find('textarea')
        expect(textarea.attributes('placeholder')).toBe('Write to general')
    })

    it('emits send event with content and file IDs', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        const textarea = wrapper.find('textarea')
        await textarea.setValue('Hello world')
        await textarea.trigger('input')

        // Find and click send button
        const sendButton = wrapper.find('[aria-label="Send message"]')
        await sendButton.trigger('click')

        expect(wrapper.emitted('send')).toBeTruthy()
        expect(wrapper.emitted('send')![0]).toEqual([{
            content: 'Hello world',
            fileIds: []
        }])
    })

    it('does not send empty messages', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        const sendButton = wrapper.find('[aria-label="Send message"]')
        expect(sendButton.attributes('disabled')).toBeDefined()
    })

    it('toggles formatting toolbar', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        // Initially visible
        expect(wrapper.find('.bg-bg-surface-2\\/50').exists()).toBe(true)

        // Toggle off
        const toggleButton = wrapper.find('[aria-label="Toggle formatting toolbar"]')
        await toggleButton.trigger('click')

        expect(wrapper.find('.bg-bg-surface-2\\/50').exists()).toBe(false)
    })

    it('applies bold formatting', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        const textarea = wrapper.find('textarea')
        await textarea.setValue('hello world')
        
        // Select text
        const element = textarea.element
        element.selectionStart = 6
        element.selectionEnd = 11
        await textarea.trigger('select')

        // Click bold button
        const boldButton = wrapper.find('[aria-label="Bold"]')
        await boldButton.trigger('click')

        // Text should be wrapped in **
        expect(textarea.element.value).toBe('hello **world**')
    })

    it('cycles through font sizes', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        const fontButton = wrapper.find('[aria-label="Toggle font size"]')
        const textarea = wrapper.find('textarea')

        // Initial: normal
        expect(textarea.classes()).toContain('text-base')

        // Click to large
        await fontButton.trigger('click')
        await flushPromises()
        expect(textarea.classes()).toContain('text-lg')

        // Click to small
        await fontButton.trigger('click')
        await flushPromises()
        expect(textarea.classes()).toContain('text-sm')

        // Click back to normal
        await fontButton.trigger('click')
        await flushPromises()
        expect(textarea.classes()).toContain('text-base')
    })

    it('shows attachment preview when files are attached', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        // Simulate file attachment
        const fileInput = wrapper.find('input[type="file"]')
        const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
        
        Object.defineProperty(fileInput.element, 'files', {
            value: [file]
        })
        
        await fileInput.trigger('change')
        await flushPromises()

        // Should show attachment preview
        expect(wrapper.text()).toContain('test.txt')
    })

    it('handles keyboard shortcuts', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        const textarea = wrapper.find('textarea')
        await textarea.setValue('hello world')
        
        // Select text
        const element = textarea.element
        element.selectionStart = 6
        element.selectionEnd = 11

        // Trigger Ctrl+B
        await textarea.trigger('keydown', {
            key: 'b',
            ctrlKey: true
        })

        expect(textarea.element.value).toBe('hello **world**')
    })

    it('prevents sending while uploading', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        // Set some content
        const textarea = wrapper.find('textarea')
        await textarea.setValue('Hello')
        
        // Simulate upload in progress by mocking state
        const vm = wrapper.vm as any
        vm.attachedFiles = [{
            file: new File(['test'], 'test.txt'),
            uploading: true,
            progress: 50
        }]
        await flushPromises()

        // Send button should be disabled
        const sendButton = wrapper.find('[aria-label="Send message"]')
        expect(sendButton.attributes('disabled')).toBeDefined()
    })

    it('clears content after sending', async () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general'
            }
        })

        const textarea = wrapper.find('textarea')
        await textarea.setValue('Hello world')

        const sendButton = wrapper.find('[aria-label="Send message"]')
        await sendButton.trigger('click')

        // After send, content should be cleared
        expect(textarea.element.value).toBe('')
    })

    it('is disabled when disabled prop is true', () => {
        const wrapper = mount(MattermostComposer, {
            props: {
                channelId: 'channel-123',
                channelName: 'general',
                disabled: true
            }
        })

        const textarea = wrapper.find('textarea')
        expect(textarea.attributes('disabled')).toBeDefined()
    })
})
