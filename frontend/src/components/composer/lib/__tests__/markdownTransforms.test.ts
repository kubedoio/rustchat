import { describe, it, expect } from 'vitest'
import {
    wrapSelection,
    toggleWrap,
    makeBold,
    makeItalic,
    makeStrikethrough,
    makeInlineCode,
    makeCodeBlock,
    makeHeading,
    makeLink,
    makeQuote,
    makeBulletedList,
    makeNumberedList,
    insertEmoji,
    insertMention,
    getWordAtCursor,
    isInCodeBlock,
    formatForPreview,
    type TextSelection
} from '../markdownTransforms'

describe('markdownTransforms', () => {
    describe('wrapSelection', () => {
        it('wraps selected text with delimiters', () => {
            const selection: TextSelection = {
                text: 'hello world',
                selectionStart: 6,
                selectionEnd: 11
            }
            const result = wrapSelection(selection, '**')
            expect(result.text).toBe('hello **world**')
            expect(result.selectionStart).toBe(8)
            expect(result.selectionEnd).toBe(13)
        })

        it('places cursor between delimiters when no selection', () => {
            const selection: TextSelection = {
                text: 'hello',
                selectionStart: 5,
                selectionEnd: 5
            }
            const result = wrapSelection(selection, '**')
            expect(result.text).toBe('hello**')
            expect(result.selectionStart).toBe(7)
            expect(result.selectionEnd).toBe(7)
        })
    })

    describe('toggleWrap', () => {
        it('adds delimiters when not present', () => {
            const selection: TextSelection = {
                text: 'hello world',
                selectionStart: 6,
                selectionEnd: 11
            }
            const result = toggleWrap(selection, '**')
            expect(result.text).toBe('hello **world**')
        })

        it('removes delimiters when present', () => {
            const selection: TextSelection = {
                text: 'hello **world**',
                selectionStart: 8,
                selectionEnd: 13
            }
            const result = toggleWrap(selection, '**')
            expect(result.text).toBe('hello world')
        })
    })

    describe('makeBold', () => {
        it('wraps text with **', () => {
            const selection: TextSelection = {
                text: 'hello world',
                selectionStart: 6,
                selectionEnd: 11
            }
            const result = makeBold(selection)
            expect(result.text).toBe('hello **world**')
        })
    })

    describe('makeItalic', () => {
        it('wraps text with *', () => {
            const selection: TextSelection = {
                text: 'hello world',
                selectionStart: 6,
                selectionEnd: 11
            }
            const result = makeItalic(selection)
            expect(result.text).toBe('hello *world*')
        })
    })

    describe('makeStrikethrough', () => {
        it('wraps text with ~~', () => {
            const selection: TextSelection = {
                text: 'hello world',
                selectionStart: 6,
                selectionEnd: 11
            }
            const result = makeStrikethrough(selection)
            expect(result.text).toBe('hello ~~world~~')
        })
    })

    describe('makeInlineCode', () => {
        it('wraps text with backticks', () => {
            const selection: TextSelection = {
                text: 'hello world',
                selectionStart: 6,
                selectionEnd: 11
            }
            const result = makeInlineCode(selection)
            expect(result.text).toBe('hello `world`')
        })
    })

    describe('makeCodeBlock', () => {
        it('wraps text in code block', () => {
            const selection: TextSelection = {
                text: 'console.log("hello")',
                selectionStart: 0,
                selectionEnd: 18
            }
            const result = makeCodeBlock(selection, 'javascript')
            expect(result.text).toBe('```javascript\nconsole.log("hello")\n```')
        })

        it('uses empty language when not specified', () => {
            const selection: TextSelection = {
                text: 'some code',
                selectionStart: 0,
                selectionEnd: 9
            }
            const result = makeCodeBlock(selection)
            expect(result.text).toBe('```\nsome code\n```')
        })
    })

    describe('makeHeading', () => {
        it('adds heading prefix', () => {
            const selection: TextSelection = {
                text: 'Title',
                selectionStart: 0,
                selectionEnd: 5
            }
            const result = makeHeading(selection, 4)
            expect(result.text).toBe('#### Title')
        })

        it('toggles heading off if already present', () => {
            const selection: TextSelection = {
                text: '#### Title',
                selectionStart: 5,
                selectionEnd: 10
            }
            const result = makeHeading(selection, 4)
            expect(result.text).toBe('Title')
        })
    })

    describe('makeLink', () => {
        it('creates link with URL', () => {
            const selection: TextSelection = {
                text: 'click here',
                selectionStart: 0,
                selectionEnd: 10
            }
            const result = makeLink(selection, 'https://example.com')
            expect(result.text).toBe('[click here](https://example.com)')
        })

        it('creates placeholder link without URL', () => {
            const selection: TextSelection = {
                text: 'click here',
                selectionStart: 0,
                selectionEnd: 10
            }
            const result = makeLink(selection)
            expect(result.text).toBe('[click text]()')
            expect(result.selectionStart).toBe(12)
            expect(result.selectionEnd).toBe(12)
        })

        it('creates link with placeholder when no selection', () => {
            const selection: TextSelection = {
                text: '',
                selectionStart: 0,
                selectionEnd: 0
            }
            const result = makeLink(selection)
            expect(result.text).toBe('[link text]()')
        })
    })

    describe('makeQuote', () => {
        it('adds quote prefix', () => {
            const selection: TextSelection = {
                text: 'quoted text',
                selectionStart: 0,
                selectionEnd: 11
            }
            const result = makeQuote(selection)
            expect(result.text).toBe('> quoted text')
        })

        it('handles multiple lines', () => {
            const selection: TextSelection = {
                text: 'line 1\nline 2',
                selectionStart: 0,
                selectionEnd: 12
            }
            const result = makeQuote(selection)
            expect(result.text).toBe('> line 1\n> line 2')
        })
    })

    describe('makeBulletedList', () => {
        it('adds bullet prefix', () => {
            const selection: TextSelection = {
                text: 'item 1',
                selectionStart: 0,
                selectionEnd: 6
            }
            const result = makeBulletedList(selection)
            expect(result.text).toBe('- item 1')
        })
    })

    describe('makeNumberedList', () => {
        it('adds numbered prefix', () => {
            const selection: TextSelection = {
                text: 'item 1',
                selectionStart: 0,
                selectionEnd: 6
            }
            const result = makeNumberedList(selection)
            expect(result.text).toBe('1. item 1')
        })
    })

    describe('insertEmoji', () => {
        it('inserts emoji code', () => {
            const selection: TextSelection = {
                text: 'Hello ',
                selectionStart: 6,
                selectionEnd: 6
            }
            const result = insertEmoji(selection, 'smile')
            expect(result.text).toBe('Hello :smile:')
            expect(result.selectionStart).toBe(13)
        })
    })

    describe('insertMention', () => {
        it('inserts @username with trailing space', () => {
            const selection: TextSelection = {
                text: 'Hello ',
                selectionStart: 6,
                selectionEnd: 6
            }
            const result = insertMention(selection, 'john')
            expect(result.text).toBe('Hello @john ')
            expect(result.selectionStart).toBe(12)
        })
    })

    describe('getWordAtCursor', () => {
        it('gets word at cursor position', () => {
            const text = 'hello world test'
            const result = getWordAtCursor(text, 10) // cursor in "world"
            expect(result.word).toBe('world')
            expect(result.start).toBe(6)
            expect(result.end).toBe(11)
        })

        it('handles cursor at start of word', () => {
            const text = 'hello world'
            const result = getWordAtCursor(text, 6)
            expect(result.word).toBe('world')
        })

        it('handles empty string', () => {
            const text = ''
            const result = getWordAtCursor(text, 0)
            expect(result.word).toBe('')
        })
    })

    describe('isInCodeBlock', () => {
        it('returns true when inside code block', () => {
            const text = '```\nsome code\n```'
            const result = isInCodeBlock(text, 10)
            expect(result).toBe(true)
        })

        it('returns false when outside code block', () => {
            const text = '```\ncode\n```\nnormal text'
            const result = isInCodeBlock(text, 18)
            expect(result).toBe(false)
        })

        it('returns false when no code block', () => {
            const text = 'normal text'
            const result = isInCodeBlock(text, 5)
            expect(result).toBe(false)
        })
    })

    describe('formatForPreview', () => {
        it('formats bold text', () => {
            const result = formatForPreview('**bold**')
            expect(result).toContain('<strong>bold</strong>')
        })

        it('formats italic text', () => {
            const result = formatForPreview('*italic*')
            expect(result).toContain('<em>italic</em>')
        })

        it('formats strikethrough text', () => {
            const result = formatForPreview('~~strikethrough~~')
            expect(result).toContain('<del>strikethrough</del>')
        })

        it('formats inline code', () => {
            const result = formatForPreview('`code`')
            expect(result).toContain('<code>code</code>')
        })

        it('formats links', () => {
            const result = formatForPreview('[link](https://example.com)')
            expect(result).toContain('<a href="https://example.com"')
        })

        it('escapes HTML', () => {
            const result = formatForPreview('<script>alert("xss")</script>')
            expect(result).toContain('&lt;script&gt;')
            expect(result).not.toContain('<script>')
        })
    })
})
