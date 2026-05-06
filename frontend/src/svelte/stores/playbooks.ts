import { get, writable } from 'svelte/store'
import { svelteApi } from './http'
import type {
    ChecklistWithTasks,
    CreatePlaybookRequest,
    Playbook,
    PlaybookChecklist,
    PlaybookFull,
    PlaybookRun,
    PlaybookTask,
    RunStatusUpdate,
    RunWithTasks,
    StartRunRequest,
    UpdatePlaybookRequest,
    UpdateRunTaskRequest,
} from '../../api/playbooks'

interface SveltePlaybooksState {
    playbooks: Playbook[]
    runs: PlaybookRun[]
    currentPlaybook: PlaybookFull | null
    currentRun: RunWithTasks | null
    statusUpdates: RunStatusUpdate[]
    loading: boolean
    saving: boolean
    error: string | null
}

const initialState: SveltePlaybooksState = {
    playbooks: [],
    runs: [],
    currentPlaybook: null,
    currentRun: null,
    statusUpdates: [],
    loading: false,
    saving: false,
    error: null,
}

function query(path: string, params: Record<string, string>): string {
    const search = new URLSearchParams(params)
    return `${path}?${search.toString()}`
}

function messageFromError(error: unknown, fallback: string): string {
    return error instanceof Error ? error.message : fallback
}

function createPlaybooksStore() {
    const { subscribe, set, update } = writable<SveltePlaybooksState>(initialState)

    async function fetchPlaybooks(teamId: string): Promise<Playbook[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<Playbook[]>(query('/playbooks', { team_id: teamId }))
            update((state) => ({ ...state, playbooks: data, loading: false }))
            return data
        } catch (error) {
            const message = messageFromError(error, 'Failed to fetch playbooks')
            update((state) => ({ ...state, loading: false, error: message }))
            throw error
        }
    }

    async function fetchRuns(teamId: string): Promise<PlaybookRun[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<PlaybookRun[]>(query('/runs', { team_id: teamId }))
            update((state) => ({ ...state, runs: data, loading: false }))
            return data
        } catch (error) {
            const message = messageFromError(error, 'Failed to fetch runs')
            update((state) => ({ ...state, loading: false, error: message }))
            throw error
        }
    }

    async function fetchPlaybook(id: string): Promise<PlaybookFull> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<PlaybookFull>(`/playbooks/${id}`)
            update((state) => ({ ...state, currentPlaybook: data, loading: false }))
            return data
        } catch (error) {
            const message = messageFromError(error, 'Failed to fetch playbook')
            update((state) => ({ ...state, loading: false, error: message }))
            throw error
        }
    }

    async function createPlaybook(teamId: string, data: CreatePlaybookRequest): Promise<Playbook> {
        update((state) => ({ ...state, saving: true, error: null }))

        try {
            const response = await svelteApi.post<Playbook>(query('/playbooks', { team_id: teamId }), data)
            update((state) => ({
                ...state,
                playbooks: [response.data, ...state.playbooks],
                saving: false,
            }))
            return response.data
        } catch (error) {
            const message = messageFromError(error, 'Failed to create playbook')
            update((state) => ({ ...state, saving: false, error: message }))
            throw error
        }
    }

    async function updatePlaybook(id: string, data: UpdatePlaybookRequest): Promise<Playbook> {
        update((state) => ({ ...state, saving: true, error: null }))

        try {
            const response = await svelteApi.put<Playbook>(`/playbooks/${id}`, data)
            update((state) => ({
                ...state,
                playbooks: state.playbooks.map((playbook) => playbook.id === id ? response.data : playbook),
                currentPlaybook: state.currentPlaybook?.id === id
                    ? { ...state.currentPlaybook, ...response.data }
                    : state.currentPlaybook,
                saving: false,
            }))
            return response.data
        } catch (error) {
            const message = messageFromError(error, 'Failed to update playbook')
            update((state) => ({ ...state, saving: false, error: message }))
            throw error
        }
    }

    async function createChecklist(playbookId: string, data: { name: string; sort_order?: number }): Promise<PlaybookChecklist> {
        const { data: checklist } = await svelteApi.post<PlaybookChecklist>(`/playbooks/${playbookId}/checklists`, data)
        return checklist
    }

    async function deleteChecklist(playbookId: string, id: string): Promise<void> {
        await svelteApi.delete(`/playbooks/${playbookId}/checklists/${id}`)
    }

    async function createTask(checklistId: string, data: { title: string; description?: string | null; sort_order?: number }): Promise<PlaybookTask> {
        const { data: task } = await svelteApi.post<PlaybookTask>(`/checklists/${checklistId}/tasks`, data)
        return task
    }

    async function updateTask(id: string, data: { title: string; description?: string | null; sort_order?: number }): Promise<PlaybookTask> {
        const { data: task } = await svelteApi.put<PlaybookTask>(`/tasks/${id}`, data)
        return task
    }

    async function deleteTask(id: string): Promise<void> {
        await svelteApi.delete(`/tasks/${id}`)
    }

    async function startRun(teamId: string, data: StartRunRequest): Promise<RunWithTasks> {
        update((state) => ({ ...state, saving: true, error: null }))

        try {
            const response = await svelteApi.post<RunWithTasks>(query('/runs', { team_id: teamId }), data)
            update((state) => ({
                ...state,
                currentRun: response.data,
                runs: [response.data.run, ...state.runs],
                saving: false,
            }))
            return response.data
        } catch (error) {
            const message = messageFromError(error, 'Failed to start run')
            update((state) => ({ ...state, saving: false, error: message }))
            throw error
        }
    }

    async function fetchRun(id: string): Promise<RunWithTasks> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<RunWithTasks>(`/runs/${id}`)
            update((state) => ({ ...state, currentRun: data, loading: false }))
            return data
        } catch (error) {
            const message = messageFromError(error, 'Failed to fetch run')
            update((state) => ({ ...state, loading: false, error: message }))
            throw error
        }
    }

    async function finishRun(id: string): Promise<PlaybookRun> {
        const { data } = await svelteApi.post<PlaybookRun>(`/runs/${id}/finish`)
        update((state) => ({
            ...state,
            currentRun: state.currentRun?.run.id === id ? { ...state.currentRun, run: data } : state.currentRun,
            runs: state.runs.map((run) => run.id === id ? data : run),
        }))
        return data
    }

    async function updateRunTask(runId: string, taskId: string, data: UpdateRunTaskRequest): Promise<void> {
        await svelteApi.put(`/runs/${runId}/tasks/${taskId}`, data)
        const current = get({ subscribe }).currentRun
        if (current?.run.id === runId) {
            await fetchRun(runId)
        }
    }

    async function fetchStatusUpdates(runId: string): Promise<RunStatusUpdate[]> {
        const { data } = await svelteApi.get<RunStatusUpdate[]>(`/runs/${runId}/updates`)
        update((state) => ({ ...state, statusUpdates: data }))
        return data
    }

    async function createStatusUpdate(runId: string, message: string): Promise<RunStatusUpdate> {
        const { data } = await svelteApi.post<RunStatusUpdate>(`/runs/${runId}/updates`, {
            message,
            is_broadcast: false,
        })
        update((state) => ({ ...state, statusUpdates: [...state.statusUpdates, data] }))
        return data
    }

    return {
        subscribe,
        fetchPlaybooks,
        fetchRuns,
        fetchPlaybook,
        createPlaybook,
        updatePlaybook,
        createChecklist,
        deleteChecklist,
        createTask,
        updateTask,
        deleteTask,
        startRun,
        fetchRun,
        finishRun,
        updateRunTask,
        fetchStatusUpdates,
        createStatusUpdate,
        reset: () => set(initialState),
    }
}

export type { ChecklistWithTasks, Playbook, PlaybookFull, PlaybookRun, RunWithTasks }
export const playbooksStore = createPlaybooksStore()
