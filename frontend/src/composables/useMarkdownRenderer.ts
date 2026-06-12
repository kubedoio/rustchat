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

  // Step 3: Post-process for Mentions (Interactive)
  // We use a strict \w+ regex and wrap in a safe span.
  // We also ensure external links have noopener noreferrer.
  const processedHtml = sanitizedHtml
    .replace(/@(\w+)/g, (_match, username) => {
      const isMe = highlightMentions && username === highlightMentions
      const highlightClass = isMe
        ? 'bg-warning/20 text-warning font-bold px-0.5 rounded border border-warning/30'
        : 'text-brand font-semibold hover:underline cursor-pointer'
      return `<span class="mention ${highlightClass}" data-username="${username}">@${username}</span>`
    })
    .replace(
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
