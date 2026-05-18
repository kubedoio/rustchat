// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

vi.mock('lucide-vue-next', () => ({
    File: { name: 'FileIcon', render: () => null },
    Download: { name: 'Download', render: () => null },
    ExternalLink: { name: 'ExternalLink', render: () => null },
}))

describe('FilePreview', () => {
    it('renders image thumbnail for image mime type', async () => {
        const FilePreview = (await import('./FilePreview.vue')).default
        const wrapper = mount(FilePreview, {
            props: {
                file: {
                    id: 'f1',
                    name: 'photo.jpg',
                    mime_type: 'image/jpeg',
                    size: 1024,
                    url: 'https://example.com/photo.jpg',
                    thumbnail_url: 'https://example.com/photo-thumb.jpg',
                },
            },
        })
        expect(wrapper.find('img').exists()).toBe(true)
        expect(wrapper.find('img').attributes('src')).toBe('https://example.com/photo-thumb.jpg')
    })

    it('renders file icon for non-image mime type', async () => {
        const FilePreview = (await import('./FilePreview.vue')).default
        const wrapper = mount(FilePreview, {
            props: {
                file: {
                    id: 'f2',
                    name: 'doc.pdf',
                    mime_type: 'application/pdf',
                    size: 2048,
                    url: 'https://example.com/doc.pdf',
                },
            },
        })
        expect(wrapper.find('img').exists()).toBe(false)
        expect(wrapper.text()).toContain('pdf')
    })

    it('emits preview when image is clicked', async () => {
        const FilePreview = (await import('./FilePreview.vue')).default
        const wrapper = mount(FilePreview, {
            props: {
                file: {
                    id: 'f1',
                    name: 'photo.jpg',
                    mime_type: 'image/jpeg',
                    size: 1024,
                    url: 'https://example.com/photo.jpg',
                },
            },
        })
        await wrapper.find('.group').trigger('click')
        expect(wrapper.emitted('preview')).toHaveLength(1)
        expect(wrapper.emitted('preview')![0]).toEqual([
            {
                id: 'f1',
                name: 'photo.jpg',
                mime_type: 'image/jpeg',
                size: 1024,
                url: 'https://example.com/photo.jpg',
            },
        ])
    })

    it('does not emit preview when non-image is clicked', async () => {
        const FilePreview = (await import('./FilePreview.vue')).default
        const wrapper = mount(FilePreview, {
            props: {
                file: {
                    id: 'f2',
                    name: 'doc.pdf',
                    mime_type: 'application/pdf',
                    size: 2048,
                    url: 'https://example.com/doc.pdf',
                },
            },
        })
        await wrapper.find('.group').trigger('click')
        expect(wrapper.emitted('preview')).toBeUndefined()
    })

    it('formats file sizes correctly', async () => {
        const FilePreview = (await import('./FilePreview.vue')).default
        const testCases = [
            { size: 500, expected: '500 B' },
            { size: 1024, expected: '1.0 KB' },
            { size: 1024 * 1024, expected: '1.0 MB' },
        ]
        for (const { size, expected } of testCases) {
            const wrapper = mount(FilePreview, {
                props: {
                    file: {
                        id: 'f',
                        name: 'test',
                        mime_type: 'text/plain',
                        size,
                        url: 'https://example.com/test',
                    },
                },
            })
            expect(wrapper.text()).toContain(expected)
        }
    })

    it('renders download link with correct href', async () => {
        const FilePreview = (await import('./FilePreview.vue')).default
        const wrapper = mount(FilePreview, {
            props: {
                file: {
                    id: 'f1',
                    name: 'photo.jpg',
                    mime_type: 'image/jpeg',
                    size: 1024,
                    url: 'https://example.com/photo.jpg',
                },
            },
        })
        const link = wrapper.find('a[download]')
        expect(link.exists()).toBe(true)
        expect(link.attributes('href')).toBe('https://example.com/photo.jpg')
    })
})
