// @vitest-environment jsdom

import { reactive } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { KnowledgeBaseSummary } from '@/api/knowledgeBases'

const kbStore = reactive({
  knowledgeBases: [] as KnowledgeBaseSummary[],
  loading: false,
  fetchKnowledgeBases: vi.fn<() => Promise<void>>(),
  deleteKnowledgeBase: vi.fn<(_: string) => Promise<void>>(),
})

vi.mock('../../features/admin/stores/knowledgeBaseStore', () => ({
  useKnowledgeBaseStore: () => kbStore,
}))

function buildKnowledgeBase(overrides: Partial<KnowledgeBaseSummary> = {}): KnowledgeBaseSummary {
  return {
    id: 'kb-1',
    name: 'Product Docs',
    description: 'Internal product knowledge',
    embedding_model: 'text-embedding-3-small',
    embedding_dimensions: 1536,
    chunk_size: 800,
    chunk_overlap: 120,
    document_count: 4,
    is_active: true,
    created_at: '2026-06-10T00:00:00Z',
    updated_at: '2026-06-10T00:00:00Z',
    ...overrides,
  }
}

async function mountView() {
  const KnowledgeBaseManagement = (await import('./KnowledgeBaseManagement.vue')).default

  return mount(KnowledgeBaseManagement, {
    global: {
      stubs: {
        CreateKnowledgeBaseModal: {
          props: ['open'],
          emits: ['close', 'created'],
          template: '<div v-if="open" data-testid="create-kb-modal" />',
        },
        EditKnowledgeBaseModal: {
          props: ['open', 'knowledgeBase'],
          emits: ['close', 'updated'],
          template:
            '<div v-if="open" data-testid="edit-kb-modal">{{ knowledgeBase?.id }} {{ knowledgeBase?.name }}</div>',
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

describe('KnowledgeBaseManagement', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    kbStore.knowledgeBases = [
      buildKnowledgeBase({
        id: 'kb-1',
        name: 'Product Docs',
        description: 'Internal product knowledge',
      }),
      buildKnowledgeBase({
        id: 'kb-2',
        name: 'Operations Runbooks',
        description: 'Incident response procedures',
        embedding_model: 'custom-embedding',
        document_count: 0,
        is_active: false,
      }),
    ]
    kbStore.loading = false
    kbStore.fetchKnowledgeBases.mockResolvedValue(undefined)
    kbStore.deleteKnowledgeBase.mockResolvedValue(undefined)
  })

  it('fetches and renders knowledge base rows on mount', async () => {
    const wrapper = await mountView()
    await flushPromises()

    expect(kbStore.fetchKnowledgeBases).toHaveBeenCalledTimes(1)
    expect(wrapper.text()).toContain('Product Docs')
    expect(wrapper.text()).toContain('Internal product knowledge')
    expect(wrapper.text()).toContain('Operations Runbooks')
    expect(wrapper.text()).toContain('text-embedding-3-small')
  })

  it('filters knowledge bases by name, description, and embedding model', async () => {
    const wrapper = await mountView()
    await flushPromises()

    await wrapper.find('input[placeholder="Search by name or model..."]').setValue('custom')

    expect(wrapper.text()).toContain('Operations Runbooks')
    expect(wrapper.text()).not.toContain('Product Docs')
  })

  it('opens create and edit modals from toolbar and row actions', async () => {
    const wrapper = await mountView()
    await flushPromises()

    await wrapper.find('button').trigger('click')
    expect(wrapper.find('[data-testid="create-kb-modal"]').exists()).toBe(true)

    await wrapper.find('button[title="Edit Knowledge Base"]').trigger('click')
    const editModal = wrapper.find('[data-testid="edit-kb-modal"]')
    expect(editModal.exists()).toBe(true)
    expect(editModal.text()).toContain('kb-1')
  })

  it('confirms knowledge base deletion through the delete modal', async () => {
    const wrapper = await mountView()
    await flushPromises()

    await wrapper.find('button[title="Delete Knowledge Base"]').trigger('click')
    expect(wrapper.find('[data-testid="base-modal"]').text()).toContain('Product Docs')

    const deleteButton = wrapper
      .findAll('button')
      .find(button => button.text().trim() === 'Delete Knowledge Base')
    expect(deleteButton?.exists()).toBe(true)

    await deleteButton!.trigger('click')
    await flushPromises()

    expect(kbStore.deleteKnowledgeBase).toHaveBeenCalledWith('kb-1')
    expect(wrapper.find('[data-testid="base-modal"]').exists()).toBe(false)
  })
})
