import { defineStore } from 'pinia'
import { ref } from 'vue'
import { playbooksApi, type Playbook, type PlaybookFull, type PlaybookRun, type RunWithTasks } from '../api/playbooks'
import { useTeamStore } from './teams'

export const usePlaybookStore = defineStore('playbooks', () => {
    const playbooks = ref<Playbook[]>([])
    const currentPlaybook = ref<PlaybookFull | null>(null)
    const runs = ref<PlaybookRun[]>([])
    const currentRun = ref<RunWithTasks | null>(null)
    const loading = ref(false)
    const teamStore = useTeamStore()

    async function fetchPlaybooks() {
        if (!teamStore.currentTeamId) return
        loading.value = true
        try {
            const response = await playbooksApi.list(teamStore.currentTeamId)
            playbooks.value = response.data
        } catch (e) {
            console.error('Failed to fetch playbooks:', e)
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

    async function createPlaybook(data: any) {
        if (!teamStore.currentTeamId) return
        try {
            const response = await playbooksApi.create(teamStore.currentTeamId, data)
            playbooks.value.push(response.data)
            return response.data
        } catch (e) {
            throw e
        }
    }

    async function updatePlaybook(id: string, data: any) {
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
            console.error('Failed to fetch runs:', e)
        }
    }

    async function startRun(playbookId: string, name: string) {
        if (!teamStore.currentTeamId) return
        try {
            const response = await playbooksApi.startRun(teamStore.currentTeamId, {
                playbook_id: playbookId,
                name
            })
            runs.value.unshift(response.data.run)
            return response.data
        } catch (e) {
            throw e
        }
    }

    return {
        playbooks,
        currentPlaybook,
        runs,
        currentRun,
        loading,
        fetchPlaybooks,
        fetchPlaybook,
        createPlaybook,
        updatePlaybook,
        fetchRuns,
        startRun
    }
})
