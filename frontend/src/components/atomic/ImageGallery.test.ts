// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('lucide-vue-next', () => ({
  X: { name: 'X', render: () => null },
  ChevronLeft: { name: 'ChevronLeft', render: () => null },
  ChevronRight: { name: 'ChevronRight', render: () => null },
  Download: { name: 'Download', render: () => null },
}))

describe('ImageGallery', () => {
  const images = [
    {
      id: 'i1',
      name: 'a.jpg',
      url: 'https://example.com/a.jpg',
      thumbnail_url: 'https://example.com/a-thumb.jpg',
    },
    {
      id: 'i2',
      name: 'b.jpg',
      url: 'https://example.com/b.jpg',
      thumbnail_url: 'https://example.com/b-thumb.jpg',
    },
    {
      id: 'i3',
      name: 'c.jpg',
      url: 'https://example.com/c.jpg',
      thumbnail_url: 'https://example.com/c-thumb.jpg',
    },
  ]

  beforeEach(() => {
    vi.clearAllMocks()
    document.body.style.overflow = ''
  })

  it('renders current image name and counter', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 0 },
    })
    expect(wrapper.text()).toContain('a.jpg')
    expect(wrapper.text()).toContain('1 of 3')
  })

  it('cycles to next image on next button click', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 0 },
    })
    await wrapper.find('button.absolute.right-4').trigger('click')
    expect(wrapper.text()).toContain('b.jpg')
    expect(wrapper.text()).toContain('2 of 3')
  })

  it('cycles from last to first image on next button click', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 2 },
    })
    await wrapper.find('button.absolute.right-4').trigger('click')
    expect(wrapper.text()).toContain('a.jpg')
    expect(wrapper.text()).toContain('1 of 3')
  })

  it('cycles to previous image on prev button click', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 1 },
    })
    await wrapper.find('button.absolute.left-4').trigger('click')
    expect(wrapper.text()).toContain('a.jpg')
    expect(wrapper.text()).toContain('1 of 3')
  })

  it('cycles from first to last image on prev button click', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 0 },
    })
    await wrapper.find('button.absolute.left-4').trigger('click')
    expect(wrapper.text()).toContain('c.jpg')
    expect(wrapper.text()).toContain('3 of 3')
  })

  it('emits close on Escape key', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 0 },
    })
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('emits close when close button is clicked', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 0 },
    })
    const closeButton = wrapper.findAll('button').find(b => b.attributes('title') === 'Close')
    expect(closeButton).toBeDefined()
    await closeButton!.trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('updates current image when thumbnail is clicked', async () => {
    const ImageGallery = (await import('./ImageGallery.vue')).default
    const wrapper = mount(ImageGallery, {
      props: { images, initialIndex: 0 },
    })
    const thumbnails = wrapper.findAll('.h-24 button')
    expect(thumbnails.length).toBe(3)
    await thumbnails[1].trigger('click')
    expect(wrapper.text()).toContain('b.jpg')
    expect(wrapper.text()).toContain('2 of 3')
  })
})
