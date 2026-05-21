export function wrapWithInlineCode(text: string): string {
  return `\`${text}\``
}

export function wrapWithCodeBlock(text: string, language: string = ''): string {
  return `\`\`\`${language}\n${text}\n\`\`\``
}

export function detectCodeLanguage(text: string): string | null {
  // Simple language detection based on common patterns
  if (text.includes('function') || text.includes('const') || text.includes('let')) {
    return 'javascript'
  }
  if (text.includes('def ') || (text.includes('import ') && text.includes(':'))) {
    return 'python'
  }
  if (text.includes('{') && text.includes('}') && text.includes(';')) {
    return 'json'
  }
  return null
}
