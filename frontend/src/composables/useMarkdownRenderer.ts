import { log } from '@/utils/log'
import { ref, computed } from 'vue'
import DOMPurify from 'dompurify'
import { replaceEmojiNames } from '../utils/emoji'

// Lazy-loaded modules
let markedInstance: typeof import('marked').marked | null = null
let hljsInstance: typeof import('highlight.js/lib/common').default | null = null
let isLoading = false
const isReady = ref(false)
const markdownSanitizeConfig = {
  ALLOWED_TAGS: [
    'p',
    'br',
    'strong',
    'em',
    'code',
    'pre',
    'span',
    'ul',
    'ol',
    'li',
    'blockquote',
    'a',
    'h1',
    'h2',
    'h3',
    'h4',
    'h5',
    'h6',
    'table',
    'thead',
    'tbody',
    'tr',
    'th',
    'td',
  ],
  ALLOWED_ATTR: ['href', 'target', 'class', 'rel', 'data-username'],
}

/**
 * Load markdown processing libraries dynamically
 */
async function loadMarkdownLibs(): Promise<void> {
  if (isReady.value || isLoading) return

  isLoading = true
  try {
    const [markedModule, hljsModule] = await Promise.all([
      import('marked'),
      import('highlight.js/lib/common'),
    ])

    // Handle both ESM named export and default export shapes across Vitest/CI environments.
    const marked = markedModule.marked ?? (markedModule as any).default ?? (markedModule as any)
    const hljs = hljsModule.default ?? (hljsModule as any)

    markedInstance = marked
    hljsInstance = hljs

    // Configure marked with syntax highlighting
    const renderer = new marked.Renderer()
    renderer.code = (code: string, infostring: string | undefined, _escaped: boolean) => {
      if (!hljsInstance) return `<pre><code>${code}</code></pre>`
      const language = infostring && hljsInstance.getLanguage(infostring) ? infostring : 'plaintext'
      const highlighted = hljsInstance.highlight(code, { language }).value
      return `<div class="code-block-wrapper"><pre><code class="hljs ${language}">${highlighted}</code><button class="copy-button">Copy</button></pre></div>`
    }

    marked.use({
      renderer,
      breaks: true,
      gfm: true,
    })

    isReady.value = true
  } catch (error) {
    log.error('Failed to load markdown libraries:', error)
  } finally {
    isLoading = false
  }
}

// Start loading immediately but non-blocking
loadMarkdownLibs()

/**
 * Wrap @mentions in interactive spans, operating only on text nodes of the
 * already-sanitized HTML.
 *
 * The previous implementation ran the `@(\w+)` regex over the whole HTML
 * string, which also matched inside attribute values and mangled real URIs
 * (e.g. `mailto:user@example.com` links or code samples). Walking text nodes
 * (and skipping code/pre/links) cannot corrupt markup by construction.
 */
function highlightMentionsInTextNodes(html: string, highlightMe?: string): string {
  const parser = new DOMParser()
  const doc = parser.parseFromString(html, 'text/html')

  const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = (node as Text).parentElement
      if (!parent) return NodeFilter.FILTER_REJECT
      // Never highlight inside code blocks, links, or existing mentions:
      // an `@` there is usually an email address or literal code, not a
      // mention.
      return parent.closest('code, pre, a, span.mention')
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT
    },
  })

  const textNodes: Text[] = []
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const text = (n as Text).nodeValue ?? ''
    if (text.includes('@')) textNodes.push(n as Text)
  }

  for (const node of textNodes) {
    const text = node.nodeValue ?? ''
    const fragment = doc.createDocumentFragment()
    let lastIndex = 0
    const mentionRegex = /@(\w+)/g
    let match: RegExpExecArray | null
    while ((match = mentionRegex.exec(text)) !== null) {
      if (match.index > lastIndex) {
        fragment.appendChild(doc.createTextNode(text.slice(lastIndex, match.index)))
      }
      const username = match[1]
      const isMe = highlightMe !== undefined && username === highlightMe
      const highlightClass = isMe
        ? 'bg-warning/20 text-warning font-bold px-0.5 rounded border border-warning/30'
        : 'text-brand font-semibold hover:underline cursor-pointer'
      const span = doc.createElement('span')
      span.className = `mention ${highlightClass}`
      span.dataset.username = username
      span.textContent = `@${username}`
      fragment.appendChild(span)
      lastIndex = match.index + match[0].length
    }
    if (lastIndex < text.length) {
      fragment.appendChild(doc.createTextNode(text.slice(lastIndex)))
    }
    node.replaceWith(fragment)
  }

  return doc.body.innerHTML
}

/**
 * Render markdown to HTML (synchronous with basic fallback)
 */
function renderMarkdownSync(markdown: string, highlightMentions?: string): string {
  if (!markdown) return ''

  // Step 0: Replace inline emoji names
  const emojified = replaceEmojiNames(markdown)

  // Step 1: Parse Markdown (if libs loaded) or use basic text
  let html: string
  if (markedInstance) {
    html = markedInstance.parse(emojified) as string
  } else {
    // Basic fallback: just escape HTML and convert newlines to <br>
    html = emojified
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\n/g, '<br>')
  }

  // Step 2: Sanitize HTML
  const sanitizedHtml = DOMPurify.sanitize(html, markdownSanitizeConfig)

  // Step 3: Post-process for Mentions (Interactive, text nodes only) and
  // ensure external links have noopener noreferrer.
  const processedHtml = highlightMentionsInTextNodes(sanitizedHtml, highlightMentions).replace(
    /<a href="([^"]+)" target="_blank">/g,
    '<a href="$1" target="_blank" rel="noopener noreferrer">'
  )

  return DOMPurify.sanitize(processedHtml, markdownSanitizeConfig)
}

/**
 * Composable for markdown rendering with lazy loading
 *
 * @example
 * const { renderMarkdown, isReady } = useMarkdownRenderer()
 * const formatted = computed(() => renderMarkdown(message.content))
 */
export function useMarkdownRenderer() {
  return {
    /**
     * Reactive state indicating if markdown libraries are loaded
     */
    isReady: computed(() => isReady.value),

    /**
     * Render markdown to safe HTML
     * Works synchronously with basic fallback, enhances when libs load
     */
    renderMarkdown: renderMarkdownSync,

    /**
     * Force reload markdown libraries (rarely needed)
     */
    reload: loadMarkdownLibs,
  }
}

// Backwards compatibility: direct export for simple usage
export { renderMarkdownSync as renderMarkdown }
