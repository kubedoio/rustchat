<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import {
  Bot,
  Plus,
  Search,
  Edit2,
  Trash2,
  AlertTriangle,
  X,
  CheckCircle,
  XCircle,
  MessageSquare,
  BarChart3,
} from 'lucide-vue-next'
import { useAgentStore } from '../../features/admin/stores/agentStore'
import { type AgentSummary } from '../../api/agents'
import CreateAgentModal from '../../components/modals/CreateAgentModal.vue'
import EditAgentModal from '../../components/modals/EditAgentModal.vue'
import BaseModal from '../../components/ui/BaseModal.vue'
import { getApiErrorMessage } from '@/core/errors/errorUtils'

const agentStore = useAgentStore()
const searchQuery = ref('')
const showCreateModal = ref(false)
const showEditModal = ref(false)
const editingAgent = ref<AgentSummary | null>(null)
const showDeleteModal = ref(false)
const deletingAgent = ref<AgentSummary | null>(null)
const deleteSubmitting = ref(false)
const deleteError = ref('')

let searchTimeout: ReturnType<typeof setTimeout>

onMounted(() => {
  agentStore.fetchAgents()
})

function fetchAgents() {
  agentStore.fetchAgents()
}

watch(searchQuery, () => {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    agentStore.fetchAgents()
  }, 300)
})

const filteredAgents = computed(() => {
  if (!searchQuery.value) return agentStore.agents
  const q = searchQuery.value.toLowerCase()
  return agentStore.agents.filter(
    a =>
      a.username.toLowerCase().includes(q) ||
      (a.display_name?.toLowerCase() ?? '').includes(q) ||
      a.title.toLowerCase().includes(q)
  )
})

function handleEdit(agent: AgentSummary) {
  editingAgent.value = agent
  showEditModal.value = true
}

function handleAnalytics(agent: AgentSummary) {
  editingAgent.value = agent
  showEditModal.value = true
}

function closeEditModal() {
  showEditModal.value = false
  editingAgent.value = null
}

function openDeleteModal(agent: AgentSummary) {
  deletingAgent.value = agent
  deleteError.value = ''
  deleteSubmitting.value = false
  showDeleteModal.value = true
}

function closeDeleteModal() {
  showDeleteModal.value = false
  deletingAgent.value = null
  deleteError.value = ''
  deleteSubmitting.value = false
}

async function confirmDeleteAgent() {
  if (!deletingAgent.value) return
  deleteSubmitting.value = true
  deleteError.value = ''
  try {
    await agentStore.deleteAgent(deletingAgent.value.id)
    closeDeleteModal()
  } catch (e: unknown) {
    deleteError.value = getApiErrorMessage(e) || 'Failed to delete agent'
  } finally {
    deleteSubmitting.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-lg font-semibold text-text-1">AI Agents</h1>
        <p class="text-text-3 text-xs mt-0.5">
          Manage AI agent configurations and channel assignments
        </p>
      </div>
      <button
        class="flex items-center gap-1.5 px-3 py-2 bg-brand hover:bg-brand/90 text-white rounded-lg text-xs font-medium transition-colors"
        @click="showCreateModal = true"
      >
        <Plus class="w-3.5 h-3.5" />
        Create Agent
      </button>
    </div>

    <!-- Filters -->
    <div class="flex items-center gap-3">
      <div class="relative flex-1 max-w-sm">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-4" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Search by name or title..."
          class="w-full pl-9 pr-3 py-2 text-xs border border-border-1 rounded-lg bg-bg-surface-1 text-text-1 focus:ring-2 focus:ring-brand/20 focus:border-brand outline-none transition-all"
        />
      </div>
    </div>

    <!-- Agents Table -->
    <div class="bg-bg-surface-1 rounded-xl border border-border-1 overflow-hidden shadow-sm">
      <table class="min-w-full divide-y divide-border-1">
        <thead class="bg-bg-surface-2">
          <tr>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Name
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Title
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Provider / Model
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Status
            </th>
            <th
              class="px-4 py-3 text-left text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Channels
            </th>
            <th
              class="px-4 py-3 text-right text-[10px] font-semibold text-text-3 uppercase tracking-wider"
            >
              Actions
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-1">
          <tr
            v-for="agent in filteredAgents"
            :key="agent.id"
            class="hover:bg-bg-surface-2/50 transition-colors"
          >
            <td class="px-4 py-3 whitespace-nowrap">
              <div class="flex items-center gap-3">
                <div
                  class="w-8 h-8 rounded-full bg-brand flex items-center justify-center text-white text-xs font-bold"
                >
                  <Bot class="w-4 h-4" />
                </div>
                <div class="min-w-0">
                  <div class="text-xs font-medium text-text-1 truncate">
                    {{ agent.display_name || agent.username }}
                  </div>
                  <div class="text-[10px] text-text-3 truncate">@{{ agent.username }}</div>
                </div>
              </div>
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-xs text-text-2">
              {{ agent.title }}
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-xs text-text-3">
              {{ agent.provider }} / {{ agent.model }}
            </td>
            <td class="px-4 py-3 whitespace-nowrap">
              <span
                v-if="agent.is_active"
                class="inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-full bg-success/10 text-success border border-success/20"
              >
                <CheckCircle class="w-3 h-3" /> Active
              </span>
              <span
                v-else
                class="inline-flex items-center gap-1 px-2 py-0.5 text-[10px] font-medium rounded-full bg-text-4/10 text-text-3 border border-text-4/20"
              >
                <XCircle class="w-3 h-3" /> Inactive
              </span>
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-xs text-text-3">
              <div class="flex items-center gap-1">
                <MessageSquare class="w-3 h-3" />
                {{ agent.channel_count }}
              </div>
            </td>
            <td class="px-4 py-3 whitespace-nowrap text-right text-xs font-medium">
              <button
                class="text-brand hover:text-brand/80 mr-2 p-1 hover:bg-brand/10 rounded transition-colors"
                title="Edit Agent"
                @click="handleEdit(agent)"
              >
                <Edit2 class="w-3.5 h-3.5" />
              </button>
              <button
                class="text-text-3 hover:text-brand mr-2 p-1 hover:bg-brand/10 rounded transition-colors"
                title="Analytics"
                @click="handleAnalytics(agent)"
              >
                <BarChart3 class="w-3.5 h-3.5" />
              </button>
              <button
                class="text-danger hover:text-danger/80 p-1 hover:bg-danger/10 rounded transition-colors"
                title="Delete Agent"
                @click="openDeleteModal(agent)"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </td>
          </tr>
          <tr v-if="filteredAgents.length === 0 && !agentStore.loading">
            <td colspan="6" class="px-4 py-12 text-center text-text-3">
              <Bot class="w-10 h-10 mx-auto mb-3 text-text-4" />
              <p class="text-xs">No agents found</p>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <CreateAgentModal
      :open="showCreateModal"
      @close="showCreateModal = false"
      @created="fetchAgents"
    />

    <EditAgentModal
      :open="showEditModal"
      :agent="editingAgent"
      @close="closeEditModal"
      @updated="fetchAgents"
    />

    <!-- Delete Modal -->
    <BaseModal v-if="deletingAgent" v-model="showDeleteModal" size="md" @close="closeDeleteModal">
      <template #header>
        <div class="flex items-start justify-between p-5 border-b border-border-1">
          <div class="flex items-start gap-3">
            <div class="rounded-lg bg-danger/10 p-2">
              <AlertTriangle class="w-4 h-4 text-danger" />
            </div>
            <div>
              <h3 class="text-sm font-semibold text-text-1">Delete Agent</h3>
              <p class="text-[10px] text-text-3 mt-0.5">
                This will permanently delete the agent and revoke its API key.
              </p>
            </div>
          </div>
          <button
            class="text-text-4 hover:text-text-2 p-1 hover:bg-bg-surface-2 rounded-lg transition-colors"
            @click="closeDeleteModal"
          >
            <X class="w-4 h-4" />
          </button>
        </div>
      </template>

      <div class="p-5 space-y-4">
        <div class="text-xs text-text-2">
          <p>
            You are about to delete agent
            <span class="font-semibold text-text-1">{{
              deletingAgent?.display_name || deletingAgent?.username
            }}</span
            >.
          </p>
          <p class="mt-1">This action cannot be undone.</p>
        </div>

        <div
          v-if="deleteError"
          class="rounded-lg border border-danger/20 bg-danger/10 px-3 py-2 text-xs text-danger"
        >
          {{ deleteError }}
        </div>
      </div>

      <template #footer>
        <div class="flex items-center justify-end gap-2">
          <button
            class="px-3 py-2 rounded-lg border border-border-1 text-text-2 text-xs font-medium hover:bg-bg-surface-2 transition-colors"
            @click="closeDeleteModal"
          >
            Cancel
          </button>
          <button
            :disabled="deleteSubmitting"
            class="px-3 py-2 rounded-lg bg-danger hover:bg-danger/90 disabled:opacity-50 text-white text-xs font-medium transition-colors"
            @click="confirmDeleteAgent"
          >
            {{ deleteSubmitting ? 'Deleting...' : 'Delete Agent' }}
          </button>
        </div>
      </template>
    </BaseModal>
  </div>
</template>
