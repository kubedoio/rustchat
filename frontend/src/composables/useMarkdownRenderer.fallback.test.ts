import { describe, it, expect, vi } from 'vitest'

// This file intentionally does NOT use vi.doMock on 'marked' or
// 'highlight.js/lib/common'. Mocking those dynamically imported, externalized
// dependencies can leak the mock into other test files that share a worker
// (observed as order-dependent failures in useMarkdownRenderer.test.ts, which
// needs the real libraries). Instead, the fallback path is exercised by
// rendering synchronously right after a fresh module import: the composable
// starts loading the markdown libraries asynchronously, so `markedInstance` is
// still null and renderMarkdown must fall back to HTML escaping.
describe('useMarkdownRenderer fallback path', () => {
  it('escapes HTML while the markdown libraries are still loading', async () => {
    vi.resetModules()

    const { useMarkdownRenderer } = await import('./useMarkdownRenderer')
    const { renderMarkdown, isReady } = useMarkdownRenderer()

    // Precondition: the async library load has not completed yet. If this
    // ever fails, the load resolved faster than the test can observe and the
    // timing assumption of this test no longer holds — do not weaken it.
    // Note: isReady === false cannot distinguish "still loading" from
    // "failed to load" (both leave the escape path active); that distinction
    // is deliberately out of scope here to avoid the leaking module mocks
    // this rewrite removed.
    expect(isReady.value).toBe(false)

    const html = renderMarkdown('<script>alert(1)</script>')
    expect(html).not.toContain('<script')
    expect(html).toContain('&lt;script&gt;')
    expect(html).toContain('&lt;/script&gt;')
  })
})
