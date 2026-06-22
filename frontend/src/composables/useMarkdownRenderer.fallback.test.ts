import { describe, it, expect, vi } from 'vitest'

describe('useMarkdownRenderer fallback path', () => {
  it('fallback path escapes HTML when markdown libs are not loaded', async () => {
    vi.resetModules()
    vi.doMock('marked', () => ({ marked: undefined }))
    vi.doMock('highlight.js/lib/common', () => ({ default: undefined }))

    const { renderMarkdown: fallbackRender } = await import('./useMarkdownRenderer')
    const html = fallbackRender('<script>alert(1)</script>')
    expect(html).not.toContain('<script')
    expect(html).toContain('&lt;script&gt;')
    expect(html).toContain('&lt;/script&gt;')

    vi.doUnmock('marked')
    vi.doUnmock('highlight.js/lib/common')
  })
})
