<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import {
  X,
  Bot,
  Type,
  Cpu,
  BrainCircuit,
  MessageSquare,
  BookOpen,
  Eye,
  EyeOff,
  Play,
  Loader2,
  BarChart3,
} from 'lucide-vue-next'
import { useAgentStore } from '../../features/admin/stores/agentStore'
import { useKnowledgeStore } from '../../features/knowledge/stores/knowledgeStore'
import agentsApi from '../../api/agents'
import adminApi, { type AdminChannel } from '../../api/admin'
import type { AgentSummary } from '../../api/agents'
import type { KnowledgeBaseSummary } from '../../api/knowledgeBases'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

const props = defineProps<{
  open: boolean
  agent: AgentSummary | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
}>()

const agentStore = useAgentStore()
const knowledgeStore = useKnowledgeStore()

const activeTab = ref('basic')
const submitting = ref(false)
const error = ref('')
const showToken = ref(false)

const channels = ref<AdminChannel[]>([])
const channelsLoading = ref(false)
const agentChannels = ref<string[]>([])
const agentChannelsLoading = ref(false)

const testMessage = ref('')
const testLoading = ref(false)
const testResult = ref<{
  response: string
  provider: string
  model: string
  latency_ms: number
} | null>(null)
const testError = ref('')

const availableKbs = ref<KnowledgeBaseSummary[]>([])
const selectedKbId = ref('')
const assignTopK = ref(5)
const assignThreshold = ref<number | null>(null)

const tabs = [
  { id: 'basic', label: 'Basic', icon: Type },
  { id: 'llm', label: 'LLM', icon: Cpu },
  { id: 'behavior', label: 'Behavior', icon: BrainCircuit },
  { id: 'channels', label: 'Channels', icon: MessageSquare },
  { id: 'knowledge', label: 'Knowledge', icon: BookOpen },
  { id: 'analytics', label: 'Analytics', icon: BarChart3 },
]

const analyticsDays = ref(7)

const form = ref({
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
  is_active: true,
})

const providers = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'ollama', label: 'Ollama' },
]

watch(
  () => activeTab.value,
  async tab => {
    if (tab === 'analytics' && props.agent) {
      await agentStore.fetchAgentAnalytics(props.agent.id, analyticsDays.value)
    }
  }
)

watch(
  () => props.agent,
  async newAgent => {
    if (newAgent) {
      activeTab.value = 'basic'
      error.value = ''
      testResult.value = null
      testError.value = ''
      testMessage.value = ''

      // Fetch full agent detail to populate form
      agentChannelsLoading.value = true
      try {
        const [detailRes, agentChRes] = await Promise.all([
          agentsApi.get(newAgent.id),
          agentsApi.listChannels(newAgent.id),
        ])
        const detail = detailRes.data
        form.value = {
          title: detail.title,
          description: detail.description || '',
          system_prompt: detail.system_prompt,
          provider: detail.provider,
          model: detail.model,
          api_token: '',
          temperature: detail.temperature,
          max_context_messages: detail.max_context_messages,
          max_output_tokens: detail.max_output_tokens,
          capabilities: { ...detail.capabilities },
          rag_enabled: detail.rag_enabled,
          rag_top_k: detail.rag_top_k,
          is_active: detail.is_active,
        }
        agentChannels.value = (agentChRes.data.channels || []).map((c: any) => c.id || c.channel_id)
      } catch {
        // Fallback to summary data
        form.value = {
          title: newAgent.title,
          description: '',
          system_prompt: '',
          provider: 'openai',
          model: newAgent.model,
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
          is_active: newAgent.is_active,
        }
        agentChannels.value = []
      } finally {
        agentChannelsLoading.value = false
      }

      // Load available channels
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
    }
  },
  { immediate: true }
)

async function submit() {
  if (!props.agent) return

  submitting.value = true
  error.value = ''

  try {
    const payload: Record<string, any> = {
      title: form.value.title,
      description: form.value.description || undefined,
      system_prompt: form.value.system_prompt,
      provider: form.value.provider,
      model: form.value.model,
      temperature: form.value.temperature,
      max_context_messages: form.value.max_context_messages,
      max_output_tokens: form.value.max_output_tokens,
      capabilities: { ...form.value.capabilities },
      rag_enabled: form.value.rag_enabled,
      rag_top_k: form.value.rag_top_k,
      is_active: form.value.is_active,
    }
    if (form.value.api_token) {
      payload.api_token = form.value.api_token
    }

    await agentStore.updateAgent(props.agent.id, payload)

    // Sync channels
    const originalIds = new Set(agentChannels.value)
    const currentIds = new Set(selectedChannelIds.value)

    const toAdd = Array.from(currentIds).filter(id => !originalIds.has(id))
    const toRemove = Array.from(originalIds).filter(id => !currentIds.has(id))

    await Promise.all([
      ...toAdd.map(id => agentsApi.addChannel(props.agent!.id, id)),
      ...toRemove.map(id => agentsApi.removeChannel(props.agent!.id, id)),
    ])

    emit('updated')
    emit('close')
  } catch (e: unknown) {
    error.value = getApiErrorMessage(e) || 'Failed to update agent'
  } finally {
    submitting.value = false
  }
}

function close() {
  error.value = ''
  testResult.value = null
  testError.value = ''
  emit('close')
}

const selectedChannelIds = computed(() => {
  // Start from agentChannels and allow user modifications
  // We track modifications via a reactive set
  return channelSelections.value
})

const channelSelections = ref<string[]>([])

watch(agentChannels, ids => {
  channelSelections.value = [...ids]
})

function toggleChannel(channelId: string) {
  const idx = channelSelections.value.indexOf(channelId)
  if (idx === -1) {
    channelSelections.value.push(channelId)
  } else {
    channelSelections.value.splice(idx, 1)
  }
}

async function runTest() {
  if (!props.agent || !testMessage.value.trim()) return
  testLoading.value = true
  testError.value = ''
  testResult.value = null
  try {
    const response = await agentsApi.test(props.agent.id, {
      message: testMessage.value.trim(),
    })
    testResult.value = response.data
  } catch (e: unknown) {
    testError.value = getApiErrorMessage(e) || 'Test failed'
  } finally {
    testLoading.value = false
  }
}

async function loadAgentKbs() {
  if (!props.agent) return
  await knowledgeStore.fetchAgentKbs(props.agent.user_id)
  await knowledgeStore.fetchKnowledgeBases()
  // Filter out already assigned KBs from available list
  const assignedIds = new Set(knowledgeStore.agentKbs.map(k => k.knowledge_base_id))
  availableKbs.value = knowledgeStore.knowledgeBases.filter(kb => !assignedIds.has(kb.id))
}

watch(
  () => props.open,
  open => {
    if (open) {
      loadAgentKbs()
    }
  }
)

async function handleAssignKb() {
  if (!props.agent || !selectedKbId.value) return
  try {
    await knowledgeStore.assignKbToAgent(props.agent.user_id, {
      knowledge_base_id: selectedKbId.value,
      top_k: assignTopK.value,
      relevance_threshold: assignThreshold.value ?? undefined,
    })
    selectedKbId.value = ''
    assignTopK.value = 5
    assignThreshold.value = null
    await loadAgentKbs()
  } catch (e) {
    error.value = getApiErrorMessage(e) || 'Failed to assign knowledge base'
  }
}

async function handleUnassignKb(kbId: string) {
  if (!props.agent) return
  try {
    await knowledgeStore.unassignKbFromAgent(props.agent.user_id, kbId)
    await loadAgentKbs()
  } catch (e) {
    error.value = getApiErrorMessage(e) || 'Failed to unassign knowledge base'
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
            <div>
              <h2 class="text-sm font-semibold text-text-1">Edit Agent</h2>
              <p v-if="agent" class="text-[10px] text-text-3">@{{ agent.username }}</p>
            </div>
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
        <div class="flex-1 overflow-y-auto p-5 space-y-4">
          <!-- Error -->
          <div
            v-if="error"
            class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs"
          >
            {{ error }}
          </div>

          <!-- Basic Tab -->
          <div v-if="activeTab === 'basic'" class="space-y-4">
            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">Title *</label>
              <input
                v-model="form.title"
                type="text"
                required
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
              />
            </div>

            <div>
              <label class="block text-xs font-medium text-text-2 mb-1.5">Description</label>
              <textarea
                v-model="form.description"
                rows="3"
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all resize-none"
              />
            </div>

            <label
              class="flex items-center gap-2.5 p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 cursor-pointer transition-colors"
            >
              <input v-model="form.is_active" type="checkbox" class="w-4 h-4 text-brand rounded" />
              <div>
                <div class="text-xs font-medium text-text-1">Active</div>
                <div class="text-[10px] text-text-3">Agent can respond to messages when active</div>
              </div>
            </label>
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
                  placeholder="Leave blank to keep existing"
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
              <label class="block text-xs font-medium text-text-2 mb-1.5">System Prompt *</label>
              <textarea
                v-model="form.system_prompt"
                rows="6"
                required
                class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all resize-none font-mono"
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
            <div
              v-if="channelsLoading || agentChannelsLoading"
              class="text-xs text-text-3 py-4 text-center"
            >
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
                  :checked="selectedChannelIds.includes(channel.id)"
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

          <!-- Knowledge Tab -->
          <div v-if="activeTab === 'knowledge'" class="space-y-4">
            <div v-if="knowledgeStore.loading" class="text-xs text-text-3 py-4 text-center">
              Loading knowledge bases...
            </div>
            <div
              v-else-if="knowledgeStore.agentKbs.length === 0"
              class="text-xs text-text-3 py-4 text-center"
            >
              No knowledge bases assigned.
            </div>
            <div v-else class="space-y-2">
              <div
                v-for="kb in knowledgeStore.agentKbs"
                :key="kb.knowledge_base_id"
                class="flex items-center justify-between p-2.5 rounded-lg border border-border-1 hover:bg-bg-surface-2 transition-colors"
              >
                <div class="min-w-0">
                  <div class="text-xs font-medium text-text-1 truncate">
                    {{ kb.knowledge_base_name }}
                  </div>
                  <div class="text-[10px] text-text-3">
                    top_k: {{ kb.top_k
                    }}<span v-if="kb.relevance_threshold !== null">
                      · threshold: {{ kb.relevance_threshold }}</span
                    >
                  </div>
                </div>
                <button
                  type="button"
                  class="px-2 py-1 text-[10px] font-medium text-danger hover:bg-danger/10 rounded-md transition-colors shrink-0 ml-2"
                  @click="handleUnassignKb(kb.knowledge_base_id)"
                >
                  Unassign
                </button>
              </div>
            </div>

            <!-- Assign new KB -->
            <div class="border-t border-border-1 pt-4 space-y-3">
              <h4 class="text-xs font-semibold text-text-1">Assign Knowledge Base</h4>
              <div class="space-y-2">
                <select
                  v-model="selectedKbId"
                  class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                >
                  <option value="">Select a knowledge base...</option>
                  <option v-for="kb in availableKbs" :key="kb.id" :value="kb.id">
                    {{ kb.name }}
                  </option>
                </select>
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <label class="block text-[10px] font-medium text-text-2 mb-1">Top K</label>
                    <input
                      v-model.number="assignTopK"
                      type="number"
                      min="1"
                      max="20"
                      class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                    />
                  </div>
                  <div>
                    <label class="block text-[10px] font-medium text-text-2 mb-1"
                      >Threshold (0-1)</label
                    >
                    <input
                      v-model.number="assignThreshold"
                      type="number"
                      min="0"
                      max="1"
                      step="0.01"
                      placeholder="Optional"
                      class="w-full px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                    />
                  </div>
                </div>
                <button
                  type="button"
                  :disabled="!selectedKbId"
                  class="w-full px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 text-white rounded-lg text-xs font-medium transition-colors"
                  @click="handleAssignKb"
                >
                  Assign
                </button>
              </div>
            </div>
          </div>

          <!-- Analytics Tab -->
          <div v-if="activeTab === 'analytics'" class="space-y-4">
            <div class="flex items-center justify-between">
              <label class="text-xs font-medium text-text-2">Time Range</label>
              <select
                v-model.number="analyticsDays"
                class="px-2 py-1.5 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                @change="
                  props.agent && agentStore.fetchAgentAnalytics(props.agent.id, analyticsDays)
                "
              >
                <option :value="7">Last 7 days</option>
                <option :value="30">Last 30 days</option>
                <option :value="90">Last 90 days</option>
              </select>
            </div>

            <div v-if="agentStore.loading" class="text-xs text-text-3 py-8 text-center">
              Loading analytics...
            </div>
            <div
              v-else-if="!agentStore.agentAnalytics"
              class="text-xs text-text-3 py-8 text-center"
            >
              No analytics data available.
            </div>
            <div v-else class="space-y-4">
              <!-- Summary Cards -->
              <div class="grid grid-cols-3 gap-3">
                <div class="p-3 rounded-lg border border-border-1 bg-bg-surface-2 space-y-1">
                  <div class="text-[10px] text-text-3 uppercase tracking-wider">Invocations</div>
                  <div class="text-sm font-semibold text-text-1">
                    {{ agentStore.agentAnalytics.summary.total_invocations }}
                  </div>
                </div>
                <div class="p-3 rounded-lg border border-border-1 bg-bg-surface-2 space-y-1">
                  <div class="text-[10px] text-text-3 uppercase tracking-wider">Tokens</div>
                  <div class="text-sm font-semibold text-text-1">
                    {{
                      agentStore.agentAnalytics.summary.total_tokens_input +
                      agentStore.agentAnalytics.summary.total_tokens_output
                    }}
                  </div>
                </div>
                <div class="p-3 rounded-lg border border-border-1 bg-bg-surface-2 space-y-1">
                  <div class="text-[10px] text-text-3 uppercase tracking-wider">Avg Latency</div>
                  <div class="text-sm font-semibold text-text-1">
                    {{ agentStore.agentAnalytics.summary.avg_latency_ms }}ms
                  </div>
                </div>
              </div>

              <!-- Feedback -->
              <div
                v-if="agentStore.agentAnalytics.feedback_stats.total_feedback > 0"
                class="p-3 rounded-lg border border-border-1 bg-bg-surface-2 space-y-1"
              >
                <div class="text-[10px] text-text-3 uppercase tracking-wider">Feedback Ratio</div>
                <div class="flex items-center gap-2">
                  <div class="text-sm font-semibold text-text-1">
                    {{ Math.round(agentStore.agentAnalytics.feedback_stats.feedback_ratio * 100) }}%
                  </div>
                  <div class="text-[10px] text-text-3">
                    ({{ agentStore.agentAnalytics.feedback_stats.total_positive }} positive /
                    {{ agentStore.agentAnalytics.feedback_stats.total_feedback }} total)
                  </div>
                </div>
              </div>

              <!-- Daily Usage Table -->
              <div class="border border-border-1 rounded-lg overflow-hidden">
                <table class="min-w-full divide-y divide-border-1">
                  <thead class="bg-bg-surface-2">
                    <tr>
                      <th
                        class="px-3 py-2 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
                      >
                        Date
                      </th>
                      <th
                        class="px-3 py-2 text-right text-[10px] font-semibold text-text-3 uppercase tracking-wider"
                      >
                        Calls
                      </th>
                      <th
                        class="px-3 py-2 text-right text-[10px] font-semibold text-text-3 uppercase tracking-wider"
                      >
                        Tokens In
                      </th>
                      <th
                        class="px-3 py-2 text-right text-[10px] font-semibold text-text-3 uppercase tracking-wider"
                      >
                        Tokens Out
                      </th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-border-1">
                    <tr v-for="day in agentStore.agentAnalytics.daily_usage" :key="day.date">
                      <td class="px-3 py-2 text-xs text-text-2">{{ day.date }}</td>
                      <td class="px-3 py-2 text-xs text-text-2 text-right">
                        {{ day.invocations }}
                      </td>
                      <td class="px-3 py-2 text-xs text-text-2 text-right">
                        {{ day.tokens_input }}
                      </td>
                      <td class="px-3 py-2 text-xs text-text-2 text-right">
                        {{ day.tokens_output }}
                      </td>
                    </tr>
                    <tr v-if="agentStore.agentAnalytics.daily_usage.length === 0">
                      <td colspan="4" class="px-3 py-4 text-center text-xs text-text-3">
                        No daily usage data
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          <!-- Test Agent Section -->
          <div class="border-t border-border-1 pt-4 space-y-3">
            <div class="flex items-center gap-2">
              <Play class="w-3.5 h-3.5 text-brand" />
              <h4 class="text-xs font-semibold text-text-1">Test Agent</h4>
            </div>
            <div class="flex gap-2">
              <input
                v-model="testMessage"
                type="text"
                placeholder="Enter a test message..."
                class="flex-1 px-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
                @keydown.enter.prevent="runTest"
              />
              <button
                type="button"
                :disabled="testLoading || !testMessage.trim()"
                class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 text-white rounded-lg text-xs font-medium transition-colors flex items-center gap-1.5"
                @click="runTest"
              >
                <Loader2 v-if="testLoading" class="w-3.5 h-3.5 animate-spin" />
                <Play v-else class="w-3.5 h-3.5" />
                {{ testLoading ? 'Testing...' : 'Test' }}
              </button>
            </div>
            <div
              v-if="testError"
              class="p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs"
            >
              {{ testError }}
            </div>
            <div
              v-if="testResult"
              class="p-3 bg-success/10 border border-success/20 rounded-lg space-y-2"
            >
              <div class="flex items-center justify-between">
                <span class="text-[10px] font-medium text-success">Response</span>
                <span class="text-[10px] text-text-3"
                  >{{ testResult.latency_ms }}ms · {{ testResult.provider }} /
                  {{ testResult.model }}</span
                >
              </div>
              <p class="text-xs text-text-1 whitespace-pre-wrap">{{ testResult.response }}</p>
            </div>
          </div>
        </div>

        <!-- Footer Actions -->
        <div class="flex justify-end gap-2 px-5 py-4 border-t border-border-1 shrink-0">
          <button
            type="button"
            class="px-3 py-2 text-text-2 hover:bg-bg-surface-2 rounded-lg text-xs font-medium transition-colors"
            @click="close"
          >
            Cancel
          </button>
          <button
            type="button"
            :disabled="submitting"
            class="px-3 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
            @click="submit"
          >
            {{ submitting ? 'Saving...' : 'Save Changes' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
