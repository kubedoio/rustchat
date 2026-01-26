<script setup lang="ts">
import { ref } from 'vue'
import { Smile, Paperclip, Send, X, File as FileIcon, Video } from 'lucide-vue-next';
import { useToast } from '../../composables/useToast';
import { filesApi, type FileUploadResponse } from '../../api/files';
import FileUploader from '../atomic/FileUploader.vue';
import EmojiPicker from '../atomic/EmojiPicker.vue';
import FormattingToolbar from './FormattingToolbar.vue';
import MarkdownPreview from './MarkdownPreview.vue';
import MentionAutocomplete from './MentionAutocomplete.vue';
import { useTeamStore } from '../../stores/teams';
import { useConfigStore } from '../../stores/config';

const emit = defineEmits(['send', 'typing', 'startCall'])
const content = ref('')
const showEmojiPicker = ref(false)
const showPreview = ref(false)
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const toast = useToast()
const teamStore = useTeamStore()
const configStore = useConfigStore()

const showMentionMenu = ref(false)
const attachedFiles = ref<{ 
    file: File; 
    uploading: boolean; 
    progress: number;
    uploaded?: FileUploadResponse 
}[]>([])
const mentionQuery = ref('')

let lastTypingEmit = 0

// Special mentions

function handleSend() {
  if (!content.value.trim() && attachedFiles.value.length === 0) return
  
  // Only send files that have finished uploading
  const fileIds = attachedFiles.value
    .filter(a => !a.uploading && a.uploaded)
    .map(a => a.uploaded!.id)

  const messageContent = content.value
  emit('send', { 
    content: messageContent, 
    file_ids: fileIds 
  })
  
  content.value = ''
  attachedFiles.value = []
  showPreview.value = false
}

function handleKeydown(e: KeyboardEvent) {
    // Ctrl/Cmd + B = Bold
    if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault()
        applyFormat('bold')
        return
    }
    // Ctrl/Cmd + I = Italic
    if ((e.ctrlKey || e.metaKey) && e.key === 'i') {
        e.preventDefault()
        applyFormat('italic')
        return
    }
    // Send on Enter
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        handleSend()
    }
}

function handleInput() {
    const now = Date.now()
    if (now - lastTypingEmit > 3000) {
        lastTypingEmit = now
        emit('typing')
    }
    
    // Check for @ mention trigger
    const textarea = textareaRef.value
    if (textarea) {
        const cursorPos = textarea.selectionStart
        const textBefore = content.value.substring(0, cursorPos)
        const lastAt = textBefore.lastIndexOf('@')
        
        if (lastAt !== -1 && (lastAt === 0 || /\s/.test(textBefore.charAt(lastAt - 1)))) {
            const query = textBefore.substring(lastAt + 1)
            if (!/\s/.test(query)) {
                mentionQuery.value = query
                showMentionMenu.value = true
                
                // Fetch members if not loaded
                if (teamStore.members.length === 0 && teamStore.currentTeamId) {
                    teamStore.fetchMembers(teamStore.currentTeamId as string)
                }
                return
            }
        }
        showMentionMenu.value = false
    }
}

function handleMentionSelect(username: string) {
    const textarea = textareaRef.value
    if (textarea) {
        const cursorPos = textarea.selectionStart
        const textBefore = content.value.substring(0, cursorPos)
        const lastAt = textBefore.lastIndexOf('@')
        
        const newContent = content.value.substring(0, lastAt) + '@' + username + ' ' + content.value.substring(cursorPos)
        content.value = newContent
        
        // Refocus and move cursor
        textarea.focus()
        const newPos = lastAt + username.length + 2 // +2 for @ and space
        setTimeout(() => {
            textarea.setSelectionRange(newPos, newPos)
        }, 0)
    }
    showMentionMenu.value = false
}

function applyFormat(type: string) {
    const textarea = textareaRef.value
    if (!textarea) return
    
    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const selectedText = content.value.substring(start, end)
    
    let before = ''
    let after = ''
    let prefix = ''
    
    switch (type) {
        case 'bold':
            before = '**'; after = '**'; break
        case 'italic':
            before = '*'; after = '*'; break
        case 'code':
            before = '`'; after = '`'; break
        case 'link':
            before = '['; after = '](url)'; break
        case 'quote':
            prefix = '> '; break
        case 'bullet':
            prefix = '- '; break
        case 'numbered':
            prefix = '1. '; break
    }
    
    if (prefix) {
        // Line-based formatting
        const lineStart = content.value.lastIndexOf('\n', start - 1) + 1
        content.value = content.value.substring(0, lineStart) + prefix + content.value.substring(lineStart)
    } else {
        // Wrap selection
        content.value = content.value.substring(0, start) + before + selectedText + after + content.value.substring(end)
        // Move cursor
        textarea.focus()
        setTimeout(() => {
            textarea.setSelectionRange(start + before.length, end + before.length)
        }, 0)
    }
}

const fileUploaderRef = ref<InstanceType<typeof FileUploader> | null>(null)

function openFilePicker() {
    fileUploaderRef.value?.openFilePicker()
}

async function handleFiles(files: File[]) {
    for (const file of files) {
        const attachment = { 
            file, 
            uploading: true, 
            progress: 0,
            uploaded: undefined as FileUploadResponse | undefined 
        }
        attachedFiles.value.push(attachment)
        
        try {
            const response = await filesApi.upload(file, undefined, (progressEvent) => {
                if (progressEvent.total) {
                    attachment.progress = Math.round((progressEvent.loaded * 100) / progressEvent.total)
                }
            })
            attachment.uploaded = response.data
            toast.success('File uploaded', file.name)
        } catch (e: any) {
            toast.error('Upload failed', e.message || 'Unknown error')
            attachedFiles.value = attachedFiles.value.filter(a => a !== attachment)
        } finally {
            attachment.uploading = false
        }
    }
}

function removeAttachment(index: number) {
    attachedFiles.value.splice(index, 1)
}

function insertEmoji(emoji: string) {
    content.value += emoji
    showEmojiPicker.value = false
}

function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B'
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}
</script>

<template>
  <FileUploader 
    ref="fileUploaderRef"
    class="p-4 pt-2 shrink-0 z-20"
    @files-selected="handleFiles"
  >

    <div class="relative rounded-2xl bg-white/10 dark:bg-slate-900/60 backdrop-blur-xl border border-white/20 dark:border-white/10 shadow-lg ring-1 ring-black/5 overflow-hidden transition-all duration-300 focus-within:ring-2 focus-within:ring-indigo-500/50 focus-within:bg-white/20 dark:focus-within:bg-slate-900/80">
        <!-- Formatting Toolbar -->
        <FormattingToolbar 
          :showPreview="showPreview"
          @format="applyFormat"
          @togglePreview="showPreview = !showPreview"
          class="border-b border-white/10 dark:border-white/5 bg-white/5"
        />
        
        <!-- Markdown Preview -->
        <div v-if="showPreview" class="p-3">
          <MarkdownPreview :content="content" />
        </div>
        
        <!-- Attached Files Preview -->
        <div v-if="attachedFiles.length > 0" class="flex flex-wrap gap-3 p-3 border-b border-white/10 dark:border-white/5 bg-white/5">
          <div 
            v-for="(attachment, index) in attachedFiles"
            :key="index"
            class="relative flex flex-col space-y-2 bg-white/5 dark:bg-slate-800/60 border border-white/10 dark:border-white/5 rounded-xl p-3 min-w-[200px] overflow-hidden group/file"
          >
            <div class="flex items-center space-x-3">
                <div class="p-2 bg-indigo-500/10 rounded-lg">
                    <FileIcon class="w-5 h-5 text-indigo-400" />
                </div>
                <div class="flex-1 min-w-0">
                    <div class="text-[13px] font-semibold text-gray-800 dark:text-slate-100 truncate pr-4">
                        {{ attachment.file.name }}
                    </div>
                    <div class="text-[11px] text-gray-500 dark:text-slate-400">
                        {{ formatFileSize(attachment.file.size) }}
                    </div>
                </div>
                <button 
                  @click="removeAttachment(index)"
                  class="absolute top-2 right-2 p-1.5 hover:bg-red-500/10 hover:text-red-500 rounded-lg text-slate-500 transition-colors opacity-0 group-hover/file:opacity-100"
                >
                  <X class="w-4 h-4" />
                </button>
            </div>

            <!-- Upload Progress -->
            <div v-if="attachment.uploading" class="space-y-1.5">
                <div class="flex justify-between text-[10px] uppercase tracking-wider font-bold">
                    <span class="text-indigo-400">Uploading...</span>
                    <span class="text-slate-500">{{ attachment.progress }}%</span>
                </div>
                <div class="h-1.5 w-full bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                    <div 
                        class="h-full bg-indigo-500 transition-all duration-300 ease-out"
                        :style="{ width: `${attachment.progress}%` }"
                    ></div>
                </div>
            </div>
            
            <div v-else-if="attachment.uploaded" class="text-[10px] text-green-500 flex items-center space-x-1 font-bold uppercase tracking-wider">
                <div class="w-1 h-1 rounded-full bg-green-500"></div>
                <span>Ready</span>
            </div>
          </div>
        </div>

        <!-- Input -->
        <div class="relative">
          <textarea 
              ref="textareaRef"
              v-model="content"
              @keydown="handleKeydown"
              @input="handleInput"
              rows="1" 
              class="w-full bg-transparent border-0 focus:ring-0 resize-none max-h-40 py-3.5 px-4 min-h-[48px] text-gray-800 dark:text-slate-200 placeholder-slate-400 placeholder:font-light" 
              placeholder="Message... (Use @ to mention)"
          ></textarea>
          
          <!-- Mention Menu -->
          <MentionAutocomplete 
            :show="showMentionMenu"
            :query="mentionQuery"
            @select="handleMentionSelect"
            @close="showMentionMenu = false"
          />
        </div>

        <!-- Footer Actions -->
        <div class="flex justify-between items-center px-2 pb-2">
            <div class="flex space-x-1 text-slate-400 relative">
                <button 
                  @click="openFilePicker"
                  class="p-2 hover:bg-indigo-500/10 hover:text-indigo-400 rounded-lg transition-colors" 
                  title="Attach file"
                >
                    <Paperclip class="w-4.5 h-4.5" />
                </button>
                <div class="relative">
                  <button 
                    @click="showEmojiPicker = !showEmojiPicker"
                    class="p-2 hover:bg-yellow-500/10 hover:text-yellow-400 rounded-lg transition-colors" 
                    title="Emoji"
                  >
                      <Smile class="w-4.5 h-4.5" />
                  </button>
                  <EmojiPicker 
                    :show="showEmojiPicker" 
                    @select="insertEmoji" 
                    @close="showEmojiPicker = false" 
                  />
                </div>

                <button
                  v-if="configStore.siteConfig.mirotalk_enabled"
                  @click="$emit('startCall')"
                  class="p-2 hover:bg-green-500/10 hover:text-green-500 rounded-lg transition-colors"
                  title="Start video call"
                >
                    <Video class="w-4.5 h-4.5" />
                </button>
            </div>
            
            <div class="flex items-center space-x-4">
                 <div class="text-[10px] text-slate-500 hidden sm:block font-medium tracking-wide">
                    <span class="bg-white/10 px-1 py-0.5 rounded text-xs mr-1">⌘B</span> 
                    <span class="bg-white/10 px-1 py-0.5 rounded text-xs">⌘I</span>
                </div>
                <button 
                    @click="handleSend"
                    :disabled="!content.trim() && attachedFiles.length === 0"
                    class="p-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-all shadow-lg shadow-indigo-600/20 active:scale-95"
                >
                    <Send class="w-4 h-4" />
                </button>
            </div>
        </div>
    </div>
  </FileUploader>
</template>
