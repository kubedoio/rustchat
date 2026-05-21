// Playbook Store - Backwards-compatible state management for playbooks

import { log } from '@/utils/log'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  playbooksApi,
  type Playbook,
  type PlaybookFull,
  type PlaybookRun,
  type RunWithTasks,
} from '../../../api/playbooks'
import { useTeamStore } from '@/features/teams/stores/teamStore'

export const usePlaybookStore = defineStore('playbookStore', () => {
  const playbooks = ref<Playbook[]>([])
  const currentPlaybook = ref<PlaybookFull | null>(null)
  const runs = ref<PlaybookRun[]>([])
  const currentRun = ref<RunWithTasks | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const teamStore = useTeamStore()

  async function fetchPlaybooks() {
    if (!teamStore.currentTeamId) return
    loading.value = true
    try {
      const response = await playbooksApi.list(teamStore.currentTeamId)
      playbooks.value = response.data
    } catch (e) {
      log.error('Failed to fetch playbooks:', e)
    } finally {
      loading.value = false
    }
  }

  async function fetchPlaybook(id: string) {
    loading.value = true
    try {
      const response = await playbooksApi.get(id)
      currentPlaybook.value = response.data
    } finally {
      loading.value = false
    }
  }

  async function createPlaybook(data: Record<string, unknown>) {
    if (!teamStore.currentTeamId) return
    try {
      const response = await playbooksApi.create(teamStore.currentTeamId, data as any)
      playbooks.value.push(response.data)
      return response.data
    } catch (e) {
      throw e
    }
  }

  async function updatePlaybook(id: string, data: Record<string, unknown>) {
    loading.value = true
    try {
      const response = await playbooksApi.update(id, data)
      // Update in list if present
      const index = playbooks.value.findIndex(p => p.id === id)
      if (index !== -1) {
        playbooks.value[index] = response.data
      }
      return response.data
    } catch (e) {
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchRuns() {
    if (!teamStore.currentTeamId) return
    try {
      const response = await playbooksApi.listRuns(teamStore.currentTeamId)
      runs.value = response.data
    } catch (e) {
      log.error('Failed to fetch runs:', e)
    }
  }

  async function startRun(playbookId: string, name: string) {
    if (!teamStore.currentTeamId) return
    try {
      const response = await playbooksApi.startRun(teamStore.currentTeamId, {
        playbook_id: playbookId,
        name,
      })
      runs.value.unshift(response.data.run)
      return response.data
    } catch (e) {
      throw e
    }
  }

  // Feature-compatible setters
  function setPlaybooks(value: Playbook[]) {
    playbooks.value = value
  }

  function addPlaybook(playbook: Playbook) {
    playbooks.value.push(playbook)
  }

  function updatePlaybookInList(playbook: Playbook) {
    const index = playbooks.value.findIndex(p => p.id === playbook.id)
    if (index !== -1) {
      playbooks.value[index] = playbook
    }
  }

  function removePlaybook(id: string) {
    playbooks.value = playbooks.value.filter(p => p.id !== id)
  }

  function setCurrentPlaybook(playbook: PlaybookFull | null) {
    currentPlaybook.value = playbook
  }

  function setRuns(value: PlaybookRun[]) {
    runs.value = value
  }

  function addRun(run: PlaybookRun) {
    runs.value.unshift(run)
  }

  function setCurrentRun(run: RunWithTasks | null) {
    currentRun.value = run
  }

  function setLoading(value: boolean) {
    loading.value = value
  }

  function setError(err: string | null) {
    error.value = err
  }

  function clearError() {
    error.value = null
  }

  return {
    playbooks,
    currentPlaybook,
    runs,
    currentRun,
    loading,
    error,
    fetchPlaybooks,
    fetchPlaybook,
    createPlaybook,
    updatePlaybook,
    fetchRuns,
    startRun,
    setPlaybooks,
    addPlaybook,
    updatePlaybookInList,
    removePlaybook,
    setCurrentPlaybook,
    setRuns,
    addRun,
    setCurrentRun,
    setLoading,
    setError,
    clearError,
  }
})
