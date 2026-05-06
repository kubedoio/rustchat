<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import { 
    Bold, Italic, Strikethrough, Heading, Link, Code, Quote, 
    List, ListOrdered, HelpCircle, Sparkles, Paperclip, 
    Smile, Send, Type, X, File as FileIcon, ChevronDown
} from 'lucide-vue-next'
import { useDraft } from './hooks/useDraft'
import { 
    makeBold, makeItalic, makeStrikethrough, makeHeading, 
    makeLink, makeInlineCode, makeQuote, 
    makeBulletedList, makeNumberedList, insertEmoji
} from './lib/markdownTransforms'
import type { TextSelection } from './lib/markdownTransforms'
import { filesApi, type FileUploadResponse } from '../../api/files'
import EmojiPicker from '../atomic/EmojiPicker.vue'
import EmojiAutocomplete from './autocomplete/EmojiAutocomplete.vue'
import ChannelAutocomplete from './autocomplete/ChannelAutocomplete.vue'
import MentionAutocomplete from '../composer/MentionAutocomplete.vue'

const props = defineProps<{
    channelId: string
    channelName: string
    placeholder?: string
    disabled?: boolean
}>()

const emit = defineEmits<{
    send: [{ content: string; fileIds: string[] }]
    typing: []
}>()

// Refs
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const emojiButtonRef = ref<HTMLElement | null>(null)
const content = ref('')
const selectionStart = ref(0)
const selectionEnd = ref(0)

// UI state
const showEmojiPicker = ref(false)
const showFormatting = ref(true)
const showSendOptions = ref(false)
const fontSize = ref<'small' | 'normal' | 'large'>('normal')

// Autocomplete state
const showMentionAutocomplete = ref(false)
const showEmojiAutocomplete = ref(false)
const showChannelAutocomplete = ref(false)
const mentionQuery = ref('')
const autocompleteTrigger = ref('') // @, #, or :
const autocompleteStartPos = ref(0)

// Files
const attachedFiles = ref<Array<{
    file: File
    uploading: boolean
    progress: number
    uploaded?: FileUploadResponse
}>>([])

// Draft persistence
const { draft, isRestored, saveDraft, clearDraft } = useDraft(props.channelId)

// Stores
const fileInputRef = ref<HTMLInputElement | null>(null)

// Placeholder text
const placeholderText = computed(() => {
    return props.placeholder || `Write to ${props.channelName}`
})

// Auto-resize textarea
function autoResize() {
    const textarea = textareaRef.value
    if (!textarea) return
    
    textarea.style.height = 'auto'
    const newHeight = Math.min(textarea.scrollHeight, 400) // Max 400px
    textarea.style.height = `${newHeight}px`
}

// Update selection tracking
function updateSelection() {
    const textarea = textareaRef.value
    if (!textarea) return
    selectionStart.value = textarea.selectionStart
    selectionEnd.value = textarea.selectionEnd
}

// Get current text selection
function getTextSelection(): TextSelection {
    return {
        text: content.value,
        selectionStart: selectionStart.value,
        selectionEnd: selectionEnd.value
    }
}

// Apply text transformation and restore focus
function applyTransform(transform: (sel: TextSelection) => { text: string; selectionStart: number; selectionEnd: number }) {
    const result = transform(getTextSelection())
    content.value = result.text
    
    nextTick(() => {
        const textarea = textareaRef.value
        if (textarea) {
            textarea.focus()
            textarea.setSelectionRange(result.selectionStart, result.selectionEnd)
            updateSelection()
            autoResize()
        }
    })
}

// Toolbar actions
function onBold() { applyTransform(makeBold) }
function onItalic() { applyTransform(makeItalic) }
function onStrikethrough() { applyTransform(makeStrikethrough) }
function onHeading() { applyTransform((sel) => makeHeading(sel, 4)) }
function onLink() { applyTransform(makeLink) }
function onInlineCode() { applyTransform(makeInlineCode) }
function onQuote() { applyTransform(makeQuote) }
function onBulletedList() { applyTransform(makeBulletedList) }
function onNumberedList() { applyTransform(makeNumberedList) }

// Handle emoji selection
function onEmojiSelect(emojiName: string) {
  const result = insertEmoji(getTextSelection(), emojiName)
  content.value = result.text
  showEmojiAutocomplete.value = false
    
    nextTick(() => {
        const textarea = textareaRef.value
        if (textarea) {
            textarea.focus()
            textarea.setSelectionRange(result.selectionStart, result.selectionEnd)
            updateSelection()
        }
    })
}

function onEmojiGlyphSelect(emoji: string) {
    const selection = getTextSelection()
    const newText = selection.text.substring(0, selection.selectionStart)
        + emoji
        + selection.text.substring(selection.selectionEnd)

    const newPos = selection.selectionStart + emoji.length
    content.value = newText
    showEmojiPicker.value = false

    nextTick(() => {
        const textarea = textareaRef.value
        if (textarea) {
            textarea.focus()
            textarea.setSelectionRange(newPos, newPos)
            updateSelection()
            autoResize()
        }
    })
}

// Handle mention selection
function onMentionSelect(username: string) {
    const before = content.value.substring(0, autocompleteStartPos.value)
    const after = content.value.substring(selectionEnd.value)
    content.value = before + '@' + username + ' ' + after
    showMentionAutocomplete.value = false
    
    nextTick(() => {
        const textarea = textareaRef.value
        if (textarea) {
            const newPos = autocompleteStartPos.value + username.length + 2
            textarea.focus()
            textarea.setSelectionRange(newPos, newPos)
            updateSelection()
        }
    })
}

// Handle channel selection
function onChannelSelect(channelName: string) {
    const before = content.value.substring(0, autocompleteStartPos.value)
    const after = content.value.substring(selectionEnd.value)
    content.value = before + '~' + channelName + ' ' + after
    showChannelAutocomplete.value = false
    
    nextTick(() => {
        const textarea = textareaRef.value
        if (textarea) {
            const newPos = autocompleteStartPos.value + channelName.length + 2
            textarea.focus()
            textarea.setSelectionRange(newPos, newPos)
            updateSelection()
        }
    })
}

// Check for autocomplete triggers
function checkAutocomplete() {
    const cursorPos = selectionStart.value
    const textBefore = content.value.substring(0, cursorPos)
    
    // Check for @ mention
    const mentionMatch = textBefore.match(/@([^\s]*)$/)
    if (mentionMatch) {
        mentionQuery.value = mentionMatch[1] ?? ''
        showMentionAutocomplete.value = true
        showEmojiAutocomplete.value = false
        showChannelAutocomplete.value = false
        autocompleteTrigger.value = '@'
        autocompleteStartPos.value = cursorPos - mentionMatch[0].length
        return
    }
    
    // Check for : emoji
    const emojiMatch = textBefore.match(/:([^\s:]*)$/)
    const emojiQuery = emojiMatch?.[1] ?? ''
    if (emojiMatch && emojiQuery.length >= 1) {
        mentionQuery.value = emojiQuery
        showEmojiAutocomplete.value = true
        showMentionAutocomplete.value = false
        showChannelAutocomplete.value = false
        autocompleteTrigger.value = ':'
        autocompleteStartPos.value = cursorPos - emojiMatch[0].length
        return
    }
    
    // Check for # channel reference (using ~ in Mattermost)
    const channelMatch = textBefore.match(/~([^\s]*)$/)
    if (channelMatch) {
        mentionQuery.value = channelMatch[1] ?? ''
        showChannelAutocomplete.value = true
        showMentionAutocomplete.value = false
        showEmojiAutocomplete.value = false
        autocompleteTrigger.value = '~'
        autocompleteStartPos.value = cursorPos - channelMatch[0].length
        return
    }
    
    // Close all autocompletes
    showMentionAutocomplete.value = false
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
}

// Handle input
function onInput() {
    updateSelection()
    autoResize()
    checkAutocomplete()
    saveDraft(content.value)
}

// Handle keydown
function onKeydown(event: KeyboardEvent) {
    updateSelection()
    
    // Handle autocomplete navigation
    if (showMentionAutocomplete.value) {
        if (event.key === 'ArrowUp') {
            event.preventDefault()
            // MentionAutocomplete handles its own navigation
            return
        }
        if (event.key === 'ArrowDown') {
            event.preventDefault()
            return
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
            event.preventDefault()
            return
        }
        if (event.key === 'Escape') {
            showMentionAutocomplete.value = false
            event.preventDefault()
            return
        }
    }
    
    if (showEmojiAutocomplete.value) {
        const emojiRef = document.querySelector('[data-emoji-autocomplete]') as any
        if (event.key === 'ArrowUp') {
            event.preventDefault()
            emojiRef?.selectPrevious()
            return
        }
        if (event.key === 'ArrowDown') {
            event.preventDefault()
            emojiRef?.selectNext()
            return
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
            event.preventDefault()
            emojiRef?.selectCurrent()
            return
        }
        if (event.key === 'Escape') {
            showEmojiAutocomplete.value = false
            event.preventDefault()
            return
        }
    }
    
    if (showChannelAutocomplete.value) {
        const channelRef = document.querySelector('[data-channel-autocomplete]') as any
        if (event.key === 'ArrowUp') {
            event.preventDefault()
            channelRef?.selectPrevious()
            return
        }
        if (event.key === 'ArrowDown') {
            event.preventDefault()
            channelRef?.selectNext()
            return
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
            event.preventDefault()
            channelRef?.selectCurrent()
            return
        }
        if (event.key === 'Escape') {
            showChannelAutocomplete.value = false
            event.preventDefault()
            return
        }
    }
    
    // Send on Enter (without shift)
    if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault()
        onSend()
        return
    }
    
    // Formatting shortcuts
    if ((event.ctrlKey || event.metaKey) && !event.altKey) {
        switch (event.key.toLowerCase()) {
            case 'b':
                event.preventDefault()
                onBold()
                return
            case 'i':
                event.preventDefault()
                onItalic()
                return
        }
    }
    
    // Ctrl+Shift shortcuts
    if ((event.ctrlKey || event.metaKey) && event.shiftKey) {
        switch (event.key.toLowerCase()) {
            case 'x':
                event.preventDefault()
                onStrikethrough()
                return
            case '7':
                event.preventDefault()
                onNumberedList()
                return
            case '8':
                event.preventDefault()
                onBulletedList()
                return
        }
    }
}

// Send message
function onSend() {
    const trimmedContent = content.value.trim()
    const fileIds = attachedFiles.value
        .filter(f => f.uploaded)
        .map(f => f.uploaded!.id)
    
    if (!trimmedContent && fileIds.length === 0) return
    if (attachedFiles.value.some(f => f.uploading)) return
    
    emit('send', { content: trimmedContent, fileIds })
    
    // Reset
    content.value = ''
    attachedFiles.value = []
    clearDraft()
    
    nextTick(() => {
        const textarea = textareaRef.value
        if (textarea) {
            textarea.style.height = 'auto'
            textarea.focus()
        }
    })
}

// File handling
async function onFileSelect(event: Event) {
    const input = event.target as HTMLInputElement
    if (!input.files?.length) return
    
    for (const file of Array.from(input.files)) {
        await uploadFile(file)
    }
    
    input.value = ''
}

async function uploadFile(file: File) {
    const fileItem: {
        file: File
        uploading: boolean
        progress: number
        uploaded?: FileUploadResponse
    } = {
        file,
        uploading: true,
        progress: 0
    }
    attachedFiles.value.push(fileItem)
    const fileIndex = attachedFiles.value.length - 1
    
    try {
        const response = await filesApi.upload(file, props.channelId, (progress) => {
            const currentFile = attachedFiles.value[fileIndex]
            if (currentFile) {
                attachedFiles.value[fileIndex] = {
                    ...currentFile,
                    progress: Math.round((progress.loaded / progress.total) * 100)
                }
            }
        })
        
        const currentFile = attachedFiles.value[fileIndex]
        if (currentFile) {
            attachedFiles.value[fileIndex] = {
                ...currentFile,
                uploaded: response.data,
                uploading: false
            }
        }
    } catch (e) {
        console.error('Upload failed:', e)
        // Remove failed upload
        const index = attachedFiles.value.indexOf(fileItem)
        if (index > -1) {
            attachedFiles.value.splice(index, 1)
        }
    }
}

function removeAttachment(index: number) {
    attachedFiles.value.splice(index, 1)
}

function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B'
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

// Drag and drop
const isDragging = ref(false)

function onDragOver(event: DragEvent) {
    event.preventDefault()
    isDragging.value = true
}

function onDragLeave(event: DragEvent) {
    event.preventDefault()
    isDragging.value = false
}

async function onDrop(event: DragEvent) {
    event.preventDefault()
    isDragging.value = false
    
    const files = event.dataTransfer?.files
    if (!files) return
    
    for (const file of Array.from(files)) {
        await uploadFile(file)
    }
}

// Paste handling
async function onPaste(event: ClipboardEvent) {
    const items = event.clipboardData?.items
    if (!items) return
    
    for (const item of Array.from(items)) {
        if (item.type.startsWith('image/')) {
            const file = item.getAsFile()
            if (file) {
                event.preventDefault()
                await uploadFile(file)
            }
        }
    }
}

// Restore draft on mount
onMounted(() => {
    if (draft.value && isRestored.value) {
        content.value = draft.value.content
        nextTick(() => {
            autoResize()
        })
    }
})

// Watch for channel changes
watch(() => props.channelId, () => {
    content.value = ''
    attachedFiles.value = []
    showMentionAutocomplete.value = false
    showEmojiAutocomplete.value = false
    showChannelAutocomplete.value = false
})

// Font size class
const fontSizeClass = computed(() => {
    switch (fontSize.value) {
        case 'small': return 'text-sm'
        case 'large': return 'text-lg'
        default: return 'text-base'
    }
})

// Can send
const canSend = computed(() => {
    const hasContent = content.value.trim().length > 0
    const hasFiles = attachedFiles.value.some(f => f.uploaded)
    const uploading = attachedFiles.value.some(f => f.uploading)
    return (hasContent || hasFiles) && !uploading
})
</script>

<template>
    <div class="relative">
        <!-- Drag overlay -->
        <div 
            v-if="isDragging"
            class="absolute inset-0 bg-primary/10 border-2 border-dashed border-primary rounded-r-2 z-50 flex items-center justify-center"
            @dragleave="onDragLeave"
            @drop="onDrop"
        >
            <span class="text-primary font-medium">Drop files to upload</span>
        </div>

        <!-- Formatting Toolbar -->
        <div 
            v-if="showFormatting"
            class="flex items-center gap-1 px-2 py-1.5 bg-bg-surface-2/50 border border-border-1 rounded-t-r-2"
        >
            <!-- Left group: Text formatting -->
            <div class="flex items-center gap-0.5">
                <button 
                    @click="onBold"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Bold (Ctrl+B)"
                    aria-label="Bold"
                >
                    <Bold class="w-4 h-4" />
                </button>
                <button 
                    @click="onItalic"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Italic (Ctrl+I)"
                    aria-label="Italic"
                >
                    <Italic class="w-4 h-4" />
                </button>
                <button 
                    @click="onStrikethrough"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Strikethrough (Ctrl+Shift+X)"
                    aria-label="Strikethrough"
                >
                    <Strikethrough class="w-4 h-4" />
                </button>
            </div>

            <div class="w-px h-5 bg-border-1 mx-1"></div>

            <!-- Middle group: Structure -->
            <div class="flex items-center gap-0.5">
                <button 
                    @click="onHeading"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Heading"
                    aria-label="Heading"
                >
                    <Heading class="w-4 h-4" />
                </button>
                <button 
                    @click="onLink"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Link"
                    aria-label="Link"
                >
                    <Link class="w-4 h-4" />
                </button>
                <button 
                    @click="onInlineCode"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Inline Code"
                    aria-label="Inline Code"
                >
                    <Code class="w-4 h-4" />
                </button>
            </div>

            <div class="w-px h-5 bg-border-1 mx-1"></div>

            <!-- Right group: Lists and quotes -->
            <div class="flex items-center gap-0.5">
                <button 
                    @click="onQuote"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Quote"
                    aria-label="Quote"
                >
                    <Quote class="w-4 h-4" />
                </button>
                <button 
                    @click="onBulletedList"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Bulleted List (Ctrl+Shift+8)"
                    aria-label="Bulleted List"
                >
                    <List class="w-4 h-4" />
                </button>
                <button 
                    @click="onNumberedList"
                    class="p-1.5 rounded hover:bg-bg-surface-1 text-text-2 hover:text-text-1 transition-standard"
                    title="Numbered List (Ctrl+Shift+7)"
                    aria-label="Numbered List"
                >
                    <ListOrdered class="w-4 h-4" />
                </button>
            </div>

            <div class="flex-1"></div>

            <!-- Help -->
            <button 
                class="p-1.5 rounded hover:bg-bg-surface-1 text-text-3 hover:text-text-1 transition-standard"
                title="Formatting Help"
                aria-label="Formatting Help"
            >
                <HelpCircle class="w-4 h-4" />
            </button>
        </div>

        <!-- Main composer area -->
        <div 
            class="relative bg-bg-surface-1 border border-border-1 rounded-b-r-2 shadow-lg"
            :class="{ 'rounded-t-r-2': !showFormatting }"
            @dragover="onDragOver"
            @drop="onDrop"
        >
            <!-- Autocompletes -->
            <MentionAutocomplete
                v-if="showMentionAutocomplete"
                :show="showMentionAutocomplete"
                :query="mentionQuery"
                @select="onMentionSelect"
                @close="showMentionAutocomplete = false"
            />
            <EmojiAutocomplete
                v-if="showEmojiAutocomplete"
                ref="emojiAutocompleteRef"
                data-emoji-autocomplete
                :show="showEmojiAutocomplete"
                :query="mentionQuery"
                @select="onEmojiSelect"
                @close="showEmojiAutocomplete = false"
            />
            <ChannelAutocomplete
                v-if="showChannelAutocomplete"
                ref="channelAutocompleteRef"
                data-channel-autocomplete
                :show="showChannelAutocomplete"
                :query="mentionQuery"
                @select="onChannelSelect"
                @close="showChannelAutocomplete = false"
            />

            <!-- Attachment preview -->
            <div v-if="attachedFiles.length > 0" class="flex flex-wrap gap-2 p-3 border-b border-border-1">
                <div 
                    v-for="(file, index) in attachedFiles"
                    :key="index"
                    class="relative flex items-center gap-2 bg-bg-surface-2 border border-border-1 rounded-r-1 px-2 py-1.5 text-sm"
                >
                    <FileIcon class="w-4 h-4 text-text-3" />
                    <span class="max-w-[150px] truncate text-text-2">{{ file.file.name }}</span>
                    <span class="text-text-3">{{ formatFileSize(file.file.size) }}</span>
                    
                    <!-- Upload progress -->
                    <div v-if="file.uploading" class="flex items-center gap-1.5">
                        <div class="w-16 h-1 bg-border-1 rounded-full overflow-hidden">
                            <div 
                                class="h-full bg-primary transition-all"
                                :style="{ width: file.progress + '%' }"
                            ></div>
                        </div>
                        <span class="text-xs text-text-3">{{ file.progress }}%</span>
                    </div>
                    
                    <span v-else class="text-xs text-success">✓</span>
                    
                    <button 
                        @click="removeAttachment(index)"
                        class="p-0.5 hover:bg-danger/10 hover:text-danger rounded transition-standard"
                        aria-label="Remove attachment"
                    >
                        <X class="w-3.5 h-3.5" />
                    </button>
                </div>
            </div>

            <!-- Textarea -->
            <textarea
                ref="textareaRef"
                v-model="content"
                :placeholder="placeholderText"
                :disabled="disabled"
                :class="fontSizeClass"
                class="w-full bg-transparent border-0 focus:ring-0 resize-none min-h-[60px] max-h-[400px] py-3 px-3 text-text-1 placeholder-text-3"
                rows="1"
                @input="onInput"
                @keydown="onKeydown"
                @keyup="updateSelection"
                @click="updateSelection"
                @paste="onPaste"
                aria-label="Message composer"
            ></textarea>

            <!-- Controls footer -->
            <div class="flex items-center justify-between px-2 pb-2">
                <!-- Left controls -->
                <div class="flex items-center gap-1">
                    <!-- Attachment -->
                    <button 
                        @click="fileInputRef?.click()"
                        class="p-1.5 rounded hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-standard"
                        title="Attach file"
                        aria-label="Attach file"
                    >
                        <Paperclip class="w-4 h-4" />
                    </button>
                    <input 
                        ref="fileInputRef"
                        type="file"
                        multiple
                        class="hidden"
                        @change="onFileSelect"
                    >

                    <!-- Emoji -->
                    <div class="relative">
                        <button 
                            ref="emojiButtonRef"
                            @click="showEmojiPicker = !showEmojiPicker"
                            class="p-1.5 rounded hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-standard"
                            title="Emoji"
                            aria-label="Insert emoji"
                        >
                            <Smile class="w-4 h-4" />
                        </button>
                        <EmojiPicker
                            v-if="showEmojiPicker"
                            :show="showEmojiPicker"
                            :anchor-el="emojiButtonRef"
                            @select="onEmojiGlyphSelect"
                            @close="showEmojiPicker = false"
                        />
                    </div>

                    <!-- Font size toggle -->
                    <button 
                        @click="fontSize = fontSize === 'normal' ? 'large' : fontSize === 'large' ? 'small' : 'normal'"
                        class="p-1.5 rounded hover:bg-bg-surface-2 text-text-3 hover:text-text-1 transition-standard"
                        :title="`Font size: ${fontSize}`"
                        aria-label="Toggle font size"
                    >
                        <Type class="w-4 h-4" />
                    </button>
                </div>

                <!-- Right controls -->
                <div class="flex items-center gap-1">
                    <!-- Formatting toggle -->
                    <button 
                        @click="showFormatting = !showFormatting"
                        class="p-1.5 rounded hover:bg-bg-surface-2 transition-standard"
                        :class="showFormatting ? 'text-primary bg-primary/10' : 'text-text-3 hover:text-text-1'"
                        title="Toggle formatting toolbar"
                        aria-label="Toggle formatting toolbar"
                    >
                        <Sparkles class="w-4 h-4" />
                    </button>

                    <!-- Send button -->
                    <div class="relative flex items-center">
                        <button 
                            @click="onSend"
                            :disabled="!canSend"
                            class="flex items-center gap-1 rounded-r-1 bg-primary px-3 py-1.5 text-brand-foreground transition-standard hover:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-50"
                            aria-label="Send message"
                        >
                            <Send class="w-4 h-4" />
                            <span class="text-sm font-medium hidden sm:inline">Send</span>
                        </button>
                        <button 
                            @click="showSendOptions = !showSendOptions"
                            class="ml-0.5 rounded-r-1 bg-primary p-1.5 text-brand-foreground transition-standard hover:bg-brand-hover"
                            aria-label="Send options"
                        >
                            <ChevronDown class="w-3 h-3" />
                        </button>

                        <!-- Send options dropdown -->
                        <div 
                            v-if="showSendOptions"
                            class="absolute bottom-full right-0 mb-1 w-48 bg-bg-surface-1 border border-border-1 rounded-r-2 shadow-xl z-50"
                        >
                            <button 
                                @click="onSend(); showSendOptions = false"
                                class="w-full px-3 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 hover:text-text-1 transition-standard"
                            >
                                Send now
                            </button>
                            <button 
                                class="w-full px-3 py-2 text-left text-sm text-text-2 hover:bg-bg-surface-2 hover:text-text-1 transition-standard opacity-50"
                                disabled
                            >
                                Send later (coming soon)
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Keyboard hints -->
        <div class="flex items-center justify-end gap-3 mt-1 px-1 text-[10px] text-text-3">
            <span><kbd class="bg-bg-surface-2 px-1 rounded">Enter</kbd> to send</span>
            <span><kbd class="bg-bg-surface-2 px-1 rounded">Shift+Enter</kbd> for newline</span>
        </div>
    </div>
</template>
