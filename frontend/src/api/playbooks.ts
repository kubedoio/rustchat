import api from './client'

export interface Playbook {
    id: string
    team_id: string
    created_by: string
    name: string
    description: string | null
    icon: string | null
    is_public: boolean
    create_channel_on_run: boolean
    channel_name_template: string | null
    default_owner_id: string | null
    webhook_enabled: boolean
    webhook_secret: string | null
    keyword_triggers: string[] | null
    is_archived: boolean
    created_at: string
    updated_at: string
}

export interface PlaybookChecklist {
    id: string
    playbook_id: string
    name: string
    sort_order: number
    created_at: string
}

export interface PlaybookTask {
    id: string
    checklist_id: string
    title: string
    description: string | null
    default_assignee_id: string | null
    assignee_role: string | null
    due_after_minutes: number | null
    slash_command: string | null
    webhook_url: string | null
    sort_order: number
    created_at: string
}

export interface ChecklistWithTasks {
    id: string // checklist id
    playbook_id: string
    name: string
    sort_order: number
    created_at: string
    tasks: PlaybookTask[]
}

export interface PlaybookFull extends Playbook {
    checklists: ChecklistWithTasks[]
}

export interface CreatePlaybookRequest {
    name: string
    description?: string
    icon?: string
    is_public?: boolean
    create_channel_on_run?: boolean
    channel_name_template?: string
}

export interface UpdatePlaybookRequest {
    name?: string
    description?: string
    icon?: string
    is_public?: boolean
    create_channel_on_run?: boolean
    channel_name_template?: string
    keyword_triggers?: string[]
}

export interface PlaybookRun {
    id: string
    playbook_id: string
    team_id: string
    channel_id: string | null
    name: string
    owner_id: string
    status: 'in_progress' | 'finished' | 'archived'
    attributes: any
    summary: string | null
    started_at: string
    finished_at: string | null
    created_at: string
}

export interface RunTask {
    id: string
    run_id: string
    task_id: string
    status: 'pending' | 'in_progress' | 'done' | 'skipped'
    assignee_id: string | null
    completed_at: string | null
    completed_by: string | null
    notes: string | null
    created_at: string
}

export interface RunProgress {
    total: number
    completed: number
    in_progress: number
    pending: number
}

export interface RunWithTasks {
    id: string // run id
    // ... run fields
    run: PlaybookRun
    tasks: RunTask[]
    progress: RunProgress
}

export interface StartRunRequest {
    playbook_id: string
    name: string
    owner_id?: string
    channel_id?: string
    attributes?: any
}

export interface UpdateRunTaskRequest {
    status?: string
    assignee_id?: string
    notes?: string
}

export interface RunStatusUpdate {
    id: string
    run_id: string
    author_id: string
    message: string
    is_broadcast: boolean
    created_at: string
}

export const playbooksApi = {
    // Playbooks
    list: (teamId: string) => api.get<Playbook[]>('/playbooks', { params: { team_id: teamId } }),
    create: (teamId: string, data: CreatePlaybookRequest) => api.post<Playbook>('/playbooks', data, { params: { team_id: teamId } }),
    get: (id: string) => api.get<PlaybookFull>(`/playbooks/${id}`),
    update: (id: string, data: UpdatePlaybookRequest) => api.put<Playbook>(`/playbooks/${id}`, data),
    delete: (id: string) => api.delete(`/playbooks/${id}`),

    // Checklists
    createChecklist: (playbookId: string, data: { name: string, sort_order?: number }) =>
        api.post<PlaybookChecklist>(`/playbooks/${playbookId}/checklists`, data),
    deleteChecklist: (playbookId: string, id: string) => api.delete(`/playbooks/${playbookId}/checklists/${id}`),

    // Tasks
    createTask: (checklistId: string, data: any) => api.post<PlaybookTask>(`/checklists/${checklistId}/tasks`, data),
    updateTask: (id: string, data: any) => api.put<PlaybookTask>(`/tasks/${id}`, data),
    deleteTask: (id: string) => api.delete(`/tasks/${id}`),

    // Runs
    listRuns: (teamId: string) => api.get<PlaybookRun[]>('/runs', { params: { team_id: teamId } }),
    startRun: (teamId: string, data: StartRunRequest) => api.post<RunWithTasks>('/runs', data, { params: { team_id: teamId } }),
    getRun: (id: string) => api.get<RunWithTasks>(`/runs/${id}`),
    finishRun: (id: string) => api.post<PlaybookRun>(`/runs/${id}/finish`),

    // Run Tasks
    updateRunTask: (runId: string, taskId: string, data: UpdateRunTaskRequest) =>
        api.put<RunTask>(`/runs/${runId}/tasks/${taskId}`, data),

    // Status Updates
    listStatusUpdates: (runId: string) => api.get<RunStatusUpdate[]>(`/runs/${runId}/updates`),
    createStatusUpdate: (runId: string, data: { message: string, is_broadcast?: boolean }) =>
        api.post<RunStatusUpdate>(`/runs/${runId}/updates`, data),
}
