// @vitest-environment jsdom

import { reactive } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { AgentSummary } from '@/api/agents'

const agentStore = reactive({
  agents: [] as AgentSummary[],
  loading: false,
  fetchAgents: vi.fn<() => Promise<void>>(),
  deleteAgent: vi.fn<(_: string) => Promise<void>>(),
})

vi.mock('../../features/admin/stores/agentStore', () => ({
  useAgentStore: () => agentStore,
}))

function buildAgent(overrides: Partial<AgentSummary> = {}): AgentSummary {
  return {
    id: 'agent-1',
    user_id: 'user-1',
    username: 'triage-bot',
    display_name: 'Triage Bot',
    avatar_url: null,
    title: 'Triage Assistant',
    provider: 'openai',
    model: 'gpt-4.1-mini',
    is_active: true,
    channel_count: 2,
    created_at: '2026-06-10T00:00:00Z',
    ...overrides,
  }
}

async function mountView() {
  const AgentManagement = (await import('./AgentManagement.vue')).default

  return mount(AgentManagement, {
    global: {
      stubs: {
        CreateAgentModal: {
          props: ['open'],
          emits: ['close', 'created'],
          template: '<div v-if="open" data-testid="create-agent-modal" />',
        },
        EditAgentModal: {
          props: ['open', 'agent'],
          emits: ['close', 'updated'],
          template:
            '<div v-if="open" data-testid="edit-agent-modal">{{ agent?.id }} {{ agent?.title }}</div>',
        },
        BaseModal: {
          props: ['modelValue', 'size'],
          emits: ['close', 'update:modelValue'],
          template:
            '<section v-if="modelValue" data-testid="base-modal"><slot name="header" /><slot /><slot name="footer" /></section>',
        },
      },
    },
  })
}

describe('AgentManagement', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    agentStore.agents = [
      buildAgent({ id: 'agent-1', username: 'triage-bot', title: 'Triage Assistant' }),
      buildAgent({
        id: 'agent-2',
        username: 'docs-bot',
        display_name: 'Docs Bot',
        title: 'Documentation Assistant',
        is_active: false,
        channel_count: 0,
      }),
    ]
    agentStore.loading = false
    agentStore.fetchAgents.mockResolvedValue(undefined)
    agentStore.deleteAgent.mockResolvedValue(undefined)
  })

  it('fetches and renders agent rows on mount', async () => {
    const wrapper = await mountView()
    await flushPromises()

    expect(agentStore.fetchAgents).toHaveBeenCalledTimes(1)
    expect(wrapper.text()).toContain('Triage Bot')
    expect(wrapper.text()).toContain('@triage-bot')
    expect(wrapper.text()).toContain('Documentation Assistant')
    expect(wrapper.text()).toContain('openai / gpt-4.1-mini')
  })

  it('filters agents by username, display name, and title', async () => {
    const wrapper = await mountView()
    await flushPromises()

    await wrapper.find('input[placeholder="Search by name or title..."]').setValue('docs')

    expect(wrapper.text()).toContain('Docs Bot')
    expect(wrapper.text()).not.toContain('Triage Bot')
  })

  it('opens create and edit modals from toolbar and row actions', async () => {
    const wrapper = await mountView()
    await flushPromises()

    await wrapper.find('button').trigger('click')
    expect(wrapper.find('[data-testid="create-agent-modal"]').exists()).toBe(true)

    await wrapper.find('button[title="Edit Agent"]').trigger('click')
    const editModal = wrapper.find('[data-testid="edit-agent-modal"]')
    expect(editModal.exists()).toBe(true)
    expect(editModal.text()).toContain('agent-1')
  })

  it('confirms agent deletion through the delete modal', async () => {
    const wrapper = await mountView()
    await flushPromises()

    await wrapper.find('button[title="Delete Agent"]').trigger('click')
    expect(wrapper.find('[data-testid="base-modal"]').text()).toContain('Triage Bot')

    const deleteButton = wrapper
      .findAll('button')
      .find(button => button.text().trim() === 'Delete Agent')
    expect(deleteButton?.exists()).toBe(true)

    await deleteButton!.trigger('click')
    await flushPromises()

    expect(agentStore.deleteAgent).toHaveBeenCalledWith('agent-1')
    expect(wrapper.find('[data-testid="base-modal"]').exists()).toBe(false)
  })
})
