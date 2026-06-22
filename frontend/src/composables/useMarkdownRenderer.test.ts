import { describe, it, expect, vi, beforeAll } from 'vitest'
import { useMarkdownRenderer } from './useMarkdownRenderer'

describe('useMarkdownRenderer XSS corpus', () => {
  const { renderMarkdown, isReady } = useMarkdownRenderer()

  beforeAll(async () => {
    // Markdown libs are loaded asynchronously; give slow CI runners plenty of time.
    await vi.waitFor(() => isReady.value === true, { timeout: 30000 })
  })

  it('removes script tags', () => {
    const html = renderMarkdown("<script>alert('xss')</script>")
    expect(html).not.toContain('<script')
    expect(html).not.toContain('</script>')
  })

  it('removes event handlers from allowed tags', () => {
    const html = renderMarkdown('<a href="#" onclick="alert(1)" onmouseover="alert(2)">link</a>')
    expect(html).not.toContain('onclick')
    expect(html).not.toContain('onmouseover')
    expect(html).not.toContain('alert(1)')
  })

  it('sanitizes javascript: URLs in links', () => {
    const html = renderMarkdown('<a href="javascript:alert(\'xss\')">click me</a>')
    expect(html).not.toContain('javascript:')
    expect(html).toContain('click me')
  })

  it('sanitizes data URLs with JS in links', () => {
    const html = renderMarkdown('<a href="data:text/html,<script>alert(1)</script>">click me</a>')
    expect(html).not.toContain('data:')
    expect(html).not.toContain('<script')
    expect(html).toContain('click me')
  })

  it('sanitizes SVG with scripts', () => {
    const html = renderMarkdown('<svg><script>alert(1)</script></svg>')
    expect(html).not.toContain('<svg')
    expect(html).not.toContain('<script')
    expect(html).not.toContain('alert(1)')
  })

  it('does not execute javascript: URLs in markdown images', () => {
    const html = renderMarkdown("![x](javascript:alert('xss'))")
    expect(html).not.toContain('javascript:')
    expect(html).not.toContain('<img')
  })

  it('removes iframe injection', () => {
    const html = renderMarkdown('<iframe src="javascript:alert(\'xss\')"></iframe>')
    expect(html).not.toContain('<iframe')
    expect(html).not.toContain('javascript:')
  })

  it('removes style attributes with expressions', () => {
    const html = renderMarkdown('<p style="background-image: url(javascript:alert(\'xss\'))">x</p>')
    expect(html).not.toContain('javascript:')
    expect(html).not.toContain('style=')
    expect(html).toContain('x')
  })

  it('escapes inline code with script tags instead of executing', () => {
    const html = renderMarkdown('`<script>alert(1)</script>`')
    expect(html).not.toContain('<script')
    expect(html).toContain('&lt;script&gt;')
    expect(html).toContain('&lt;/script&gt;')
  })

  it('does not produce executable HTML from mentions with script tags', () => {
    const html = renderMarkdown('@<script>alert(1)</script>')
    expect(html).not.toContain('<script')
    expect(html).not.toContain('alert(1)')
  })
})
