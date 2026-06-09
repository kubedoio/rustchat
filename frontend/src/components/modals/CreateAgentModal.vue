<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  X,
  Bot,
  Type,
  Mail,
  Settings,
  Cpu,
  BrainCircuit,
  MessageSquare,
  Eye,
  EyeOff,
} from 'lucide-vue-next'
import { useAgentStore } from '../../features/admin/stores/agentStore'
import adminApi, { type AdminChannel } from '../../api/admin'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'created'): void
}>()

const agentStore = useAgentStore()

const activeTab = ref('basic')
const submitting = ref(false)
const error = ref('')
const showToken = ref(false)

const channels = ref<AdminChannel[]>([])
const channelsLoading = ref(false)

const tabs = [
  { id: 'basic', label: 'Basic', icon: Type },
  { id: 'llm', label: 'LLM', icon: Cpu },
  { id: 'behavior', label: 'Behavior', icon: BrainCircuit },
  { id: 'channels', label: 'Channels', icon: MessageSquare },
]

const form = ref({
  username: '',
  email: '',
  display_name: '',
  title: '',
  description: '',
  system_prompt: '',
  provider: 'openai',
  model: '',
  api_token: '',
  temperature: 0.7,
  max_context_messages: 10,
  max_output_tokens: 1024,
  capabilities: {
    respond_to_mentions: true,
    respond_to_all: false,
    use_memory: true,
    use_rag: false,
  },
  rag_enabled: false,
  rag_top_k: 5,
  channel_ids: [] as string[],
})

const providers = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'ollama', label: 'Ollama' },
]

const isValid = computed(() => {
  return (
    form.value.username.length >= 2 &&
    form.value.email.includes('@') &&
    form.value.title.length > 0 &&
    form.value.system_prompt.length > 0
  )
})

onMounted(async () => {
  if (channels.value.length === 0) {
    channelsLoading.value = true
    try {
      const response = await adminApi.listChannels({ per_page: 100 })
      channels.value = response.data.channels
    } catch {
      // ignore
    } finally {
      channelsLoading.value = false
    }
  }
})

async function submit() {
  if (!isValid.value) return

  submitting.value = true
  error.value = ''

  try {
    await agentStore.createAgent({
      username: form.value.username,
      email: form.value.email,
      display_name: form.value.display_name || undefined,
      title: form.value.title,
      description: form.value.description || undefined,
      system_prompt: form.value.system_prompt,
      provider: form.value.provider,
      model: form.value.model,
      api_token: form.value.api_token || undefined,
      temperature: form.value.temperature,
      max_context_messages: form.value.max_context_messages,
      max_output_tokens: form.value.max_output_tokens,
      capabilities: { ...form.value.capabilities },
      rag_enabled: form.value.rag_enabled,
      rag_top_k: form.value.rag_top_k,
      channel_ids: form.value.channel_ids.length > 0 ? form.value.channel_ids : undefined,
    })

    // Reset form
    form.value = {
      username: '',
      email: '',
      display_name: '',
      title: '',
      description: '',
      system_prompt: '',
      provider: 'openai',
      model: '',
      api_token: '',
      temperature: 0.7,
      max_context_messages: 10,
      max_output_tokens: 1024,
      capabilities: {
        respond_to_mentions: true,
        respond_to_all: false,
        use_memory: true,
        use_rag: false,
      },
      rag_enabled: false,
      rag_top_k: 5,
      channel_ids: [],
    }
    activeTab.value = 'basic'

    emit('created')
    emit('close')
  } catch (e: unknown) {
    error.value = getApiErrorMessage(e) || 'Failed to create agent'
  } finally {
    submitting.value = false
  }
}

function close() {
  error.value = ''
  emit('close')
}

function toggleChannel(channelId: string) {
  const idx = form.value.channel_ids.indexOf(channelId)
  if (idx === -1) {
    form.value.channel_ids.push(channelId)
  } else {
    form.value.channel_ids.splice(idx, 1)
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="close"></div>

      <!-- Modal -->
      <div
        class="relative bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh]"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 py-4 border-b border-border-1 shrink-0">
          <div class="flex items-center gap-2.5">
            <div class="rounded-lg bg-brand/10 p-1.5">
              <Bot class="w-4 h-4 text-brand" />
            </div>
            <h2 class="text-sm font-semibold text-text-1">Create Agent</h2>
          </div>
          <button class="p-1.5 hover:bg-bg-surface-2 rounded-lg transition-colors" @click="close">
            <X class="w-4 h-4 text-text-4" />
          </button>
        </div>

        <!-- Tabs -->
        <div class="flex border-b border-border-1 bg-bg-surface-2 shrink-0">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="flex items-center gap-1.5 px-4 py-2.5 text-xs font-medium transition-colors border-b-2"
            :class="
              activeTab === tab.id
                ? 'border-brand text-brand bg-brand/5'
                : 'border-transparent text-text-3 hover:text-text-1 hover:bg-bg-surface-1'
            "
            @click="activeTab = tab.id"
          >
            <component :is="tab.icon" class="w-3.5 h-3.5" />
            {{ tab.label }}
          </button>
        </div>

        <!-- Form -->
        <form class="flex-1 overflow-y-auto p-5 space-y-4" @submit.prevent="submit">
          <!-- Error -->
          <div
            v-if="error"
            class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs"
          >
            {{ error }}
          </div>

          <!-- Basic Tab -->
          <div v-if="activeTab === 'basic'" class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                  <Type class="w-3.5 h-3.5 text-text-4" />
                  Username *
                </label>
                <input
                  v-model="form.username"
                  type="text"
                  required
                  minlength="2"
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                  placeholder="agent-name"
                />
              </div>
              <div>
                <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                  <Mail class="w-3.5 h-3.5 text-text-4" />
                  Email *
                </label>
                <input
                  v-model="form.email"
                  type="email"
                  required
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                  placeholder="agent@example.com"
                />
              </div>
            </div>

            <div>
              <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                <Settings class="w-3.5 h-3.5 text-text-4" />
                Display Name
              </label>
              <input
                v-model="form.display_name"
                type="text"
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                placeholder="Agent Display Name"
              />
            </div>

            <div>
              <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                <Type class="w-3.5 h-3.5 text-text-4" />
                Title *
              </label>
              <input
                v-model="form.title"
                type="text"
                required
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                placeholder="Support Agent"
              />
            </div>

            <div>
              <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                <Settings class="w-3.5 h-3.5 text-text-4" />
                Description
              </label>
              <textarea
                v-model="form.description"
                rows="3"
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all resize-none"
                placeholder="Brief description of this agent's purpose..."
              />
            </div>
          </div>

          <!-- LLM Tab -->
          <div v-if="activeTab === 'llm'" class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-xs font-medium text-text-2 mb-1.5">Provider</label>
                <select
                  v-model="form.provider"
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                >
                  <option v-for="p in providers" :key="p.value" :value="p.value">
                    {{ p.label }}
                  </option>
                </select>
              </div>
              <div>
                <label class="block text-xs font-medium text-text-2 mb-1.5">Model</label>
                <input
                  v-model="form.model"
                  type="text"
                  required
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                  placeholder="gpt-4o"
                />
              </div>
            </div>

            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">API Token</label>
              <div class="relative">
                <input
                  v-model="form.api_token"
                  :type="showToken ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                  placeholder="sk-..."
                />
                <button
                  type="button"
                  class="absolute right-3 top-1/2 -translate-y-1/2 text-text-4 hover:text-text-2 p-1"
                  @click="showToken = !showToken"
                >
                  <Eye v-if="!showToken" class="w-3.5 h-3.5" />
                  <EyeOff v-else class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">
                Temperature: {{ form.temperature }}
              </label>
              <input
                v-model.number="form.temperature"
                type="range"
                min="0"
                max="2"
                step="0.1"
                class="w-full accent-brand"
              />
              <div class="flex justify-between text-[10px] text-text-4 mt-1">
                <span>0</span>
                <span>1</span>
                <span>2</span>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-xs font-medium text-text-2 mb-1.5">
                  Max Context Messages
                </label>
                <input
                  v-model.number="form.max_context_messages"
                  type="number"
                  min="1"
                  max="100"
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                />
              </div>
              <div>
                <label class="block text-xs font-medium text-text-2 mb-1.5">
                  Max Output Tokens
                </label>
                <input
                  v-model.number="form.max_output_tokens"
                  type="number"
                  min="1"
                  max="8192"
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                />
              </div>
            </div>
          </div>

          <!-- Behavior Tab -->
          <div v-if="activeTab === 'behavior'" class="space-y-4">
            <div>
              <label class="flex items-center gap-1.5 text-xs font-medium text-text-2 mb-1.5">
                <BrainCircuit class="w-3.5 h-3.5 text-text-4" />
                System Prompt *
              </label>
              <textarea
                v-model="form.system_prompt"
                rows="6"
                required
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all resize-none font-mono"
                placeholder="You are a helpful assistant..."
              />
            </div>

            <div>
              <label class="block text-xs font-medium text-text-2 mb-2">Capabilities</label>
              <div class="space-y-2">
                <label
                  class="flex items-center gap-2.5 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
                >
                  <input
                    v-model="form.capabilities.respond_to_mentions"
                    type="checkbox"
                    class="w-4 h-4 text-brand rounded"
                  />
                  <div>
                    <div class="text-xs font-medium text-text-1">Respond to Mentions</div>
                    <div class="text-[10px] text-text-3">Reply when @mentioned in channels</div>
                  </div>
                </label>
                <label
                  class="flex items-center gap-2.5 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
                >
                  <input
                    v-model="form.capabilities.respond_to_all"
                    type="checkbox"
                    class="w-4 h-4 text-brand rounded"
                  />
                  <div>
                    <div class="text-xs font-medium text-text-1">Respond to All Messages</div>
                    <div class="text-[10px] text-text-3">
                      Reply to every message in assigned channels
                    </div>
                  </div>
                </label>
                <label
                  class="flex items-center gap-2.5 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
                >
                  <input
                    v-model="form.capabilities.use_memory"
                    type="checkbox"
                    class="w-4 h-4 text-brand rounded"
                  />
                  <div>
                    <div class="text-xs font-medium text-text-1">Use Memory</div>
                    <div class="text-[10px] text-text-3">
                      Remember past conversations with users
                    </div>
                  </div>
                </label>
                <label
                  class="flex items-center gap-2.5 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
                >
                  <input
                    v-model="form.capabilities.use_rag"
                    type="checkbox"
                    class="w-4 h-4 text-brand rounded"
                  />
                  <div>
                    <div class="text-xs font-medium text-text-1">Use RAG</div>
                    <div class="text-[10px] text-text-3">
                      Retrieve relevant context from knowledge base
                    </div>
                  </div>
                </label>
              </div>
            </div>

            <div v-if="form.capabilities.use_rag" class="grid grid-cols-2 gap-4">
              <label
                class="flex items-center gap-2 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
              >
                <input
                  v-model="form.rag_enabled"
                  type="checkbox"
                  class="w-4 h-4 text-brand rounded"
                />
                <div>
                  <div class="text-xs font-medium text-text-1">RAG Enabled</div>
                </div>
              </label>
              <div>
                <label class="block text-xs font-medium text-text-2 mb-1.5">RAG Top K</label>
                <input
                  v-model.number="form.rag_top_k"
                  type="number"
                  min="1"
                  max="20"
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                />
              </div>
            </div>
          </div>

          <!-- Channels Tab -->
          <div v-if="activeTab === 'channels'" class="space-y-4">
            <div v-if="channelsLoading" class="text-xs text-text-3 py-4 text-center">
              Loading channels...
            </div>
            <div v-else-if="channels.length === 0" class="text-xs text-text-3 py-4 text-center">
              No channels available.
            </div>
            <div v-else class="space-y-1 max-h-64 overflow-y-auto">
              <label
                v-for="channel in channels"
                :key="channel.id"
                class="flex items-center gap-2.5 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
              >
                <input
                  :checked="form.channel_ids.includes(channel.id)"
                  type="checkbox"
                  class="w-4 h-4 text-brand rounded"
                  @change="toggleChannel(channel.id)"
                />
                <div class="min-w-0">
                  <div class="text-xs font-medium text-text-1 truncate">
                    {{ channel.display_name || channel.name }}
                  </div>
                  <div class="text-[10px] text-text-3 truncate">
                    #{{ channel.name }} · {{ channel.channel_type }}
                  </div>
                </div>
              </label>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex justify-end gap-2 pt-4 border-t border-border-1">
            <button
              type="button"
              class="px-3 py-2 text-text-2 hover:bg-bg-surface-2 rounded-lg text-xs font-medium transition-colors"
              @click="close"
            >
              Cancel
            </button>
            <button
              type="submit"
              :disabled="!isValid || submitting"
              class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
            >
              {{ submitting ? 'Creating...' : 'Create Agent' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
