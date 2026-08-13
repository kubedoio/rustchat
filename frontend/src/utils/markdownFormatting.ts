export function wrapWithInlineCode(text: string): string {
  return `\`${text}\``
}

export function wrapWithCodeBlock(text: string, language: string = ''): string {
  return `\`\`\`${language}\n${text}\n\`\`\``
}
