import { marked } from 'marked'
import DOMPurify from 'dompurify'
import hljs from 'highlight.js/lib/common'
import 'highlight.js/styles/github-dark.css'
import { replaceEmojiNames } from './emoji'

const renderer = new marked.Renderer()
// @ts-expect-error marked v12 passes object but @types/marked v5 types old 3-arg signature
renderer.code = ({ text, lang }: { text: string; lang?: string }) => {
  const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext'
  const highlighted = hljs.highlight(text, { language }).value
  return `<pre><code class="hljs ${language}">${highlighted}</code></pre>`
}

marked.use({ renderer, breaks: true, gfm: true })

export function renderMarkdown(markdown: string, highlightMentions?: string): string {
  if (!markdown) return ''
  const emojified = replaceEmojiNames(markdown)
  const html = marked.parse(emojified) as string
  const sanitizedHtml = DOMPurify.sanitize(html, {
    ALLOWED_TAGS: ['p','br','strong','em','code','pre','span','ul','ol','li','blockquote','a','h1','h2','h3','h4','h5','h6','table','thead','tbody','tr','th','td'],
    ALLOWED_ATTR: ['href','target','class','style','rel']
  })
  const processedHtml = sanitizedHtml.replace(
    /@(\w+)/g,
    (_match, username) => {
      const isMe = highlightMentions && username === highlightMentions
      const highlightClass = isMe
        ? 'bg-warning/20 text-warning font-bold px-0.5 rounded border border-warning/30'
        : 'text-brand font-semibold hover:underline cursor-pointer'
      return `<span class="mention ${highlightClass}" data-username="${username}">@${username}</span>`
    }
  )
  return processedHtml
}
