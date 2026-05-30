// Playbook Repository - Data access for playbooks

import { playbooksApi } from '../../../api/playbooks'
import type { Playbook, PlaybookFull, PlaybookRun, RunWithTasks } from '../../../api/playbooks'
import type { TeamId } from '../../../core/entities/Team'
import { withRetry } from '../../../core/services/retry'

export interface CreatePlaybookRequest {
  name: string
  description?: string
  icon?: string
  isPublic?: boolean
}

export interface StartRunRequest {
  playbookId: string
  name: string
}

export const playbookRepository = {
  // List playbooks for team
  async listByTeam(teamId: TeamId): Promise<Playbook[]> {
    return withRetry(async () => {
      const response = await playbooksApi.list(teamId)
      return response.data
    })
  },

  // Get single playbook
  async getById(id: string): Promise<PlaybookFull> {
    return withRetry(async () => {
      const response = await playbooksApi.get(id)
      return response.data
    })
  },

  // Create playbook
  async create(teamId: TeamId, data: CreatePlaybookRequest): Promise<Playbook> {
    return withRetry(async () => {
      const response = await playbooksApi.create(teamId, data)
      return response.data
    })
  },

  // Update playbook
  async update(id: string, data: Partial<CreatePlaybookRequest>): Promise<Playbook> {
    return withRetry(async () => {
      const response = await playbooksApi.update(id, data)
      return response.data
    })
  },

  // Delete playbook
  async delete(id: string): Promise<void> {
    await withRetry(() => playbooksApi.delete(id))
  },

  // List runs for team
  async listRuns(teamId: TeamId): Promise<PlaybookRun[]> {
    return withRetry(async () => {
      const response = await playbooksApi.listRuns(teamId)
      return response.data
    })
  },

  // Get run details
  async getRun(id: string): Promise<RunWithTasks> {
    return withRetry(async () => {
      const response = await playbooksApi.getRun(id)
      return response.data
    })
  },

  // Start new run
  async startRun(teamId: TeamId, request: StartRunRequest): Promise<{ run: PlaybookRun }> {
    return withRetry(async () => {
      const response = await playbooksApi.startRun(teamId, {
        playbook_id: request.playbookId,
        name: request.name,
      })
      return response.data
    })
  },

  // Update run task
  async updateRunTask(runId: string, taskId: string, status: string): Promise<void> {
    await withRetry(() => playbooksApi.updateRunTask(runId, taskId, { status }))
  },
}
