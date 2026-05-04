// Playbook Service - Business logic for playbooks

import { playbookRepository, type CreatePlaybookRequest, type StartRunRequest } from '../repositories/playbookRepository'
import type { Playbook, PlaybookRun } from '../../../api/playbooks'
import type { TeamId } from '../../../core/entities/Team'
import { usePlaybookStore } from '../stores/playbookStore'
import { AppError } from '../../../core/errors/AppError'

class PlaybookService {
  private get store() {
    return usePlaybookStore()
  }

  // Playbooks
  async loadPlaybooks(teamId: TeamId): Promise<void> {
    this.store.setLoading(true)
    try {
      const playbooks = await playbookRepository.listByTeam(teamId)
      this.store.setPlaybooks(playbooks)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to load playbooks'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  async loadPlaybook(id: string): Promise<void> {
    this.store.setLoading(true)
    try {
      const playbook = await playbookRepository.getById(id)
      this.store.setCurrentPlaybook(playbook)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to load playbook'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  async createPlaybook(teamId: TeamId, data: CreatePlaybookRequest): Promise<Playbook> {
    try {
      const playbook = await playbookRepository.create(teamId, data)
      this.store.addPlaybook(playbook)
      return playbook
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to create playbook'
      )
      throw error
    }
  }

  async updatePlaybook(id: string, data: Partial<CreatePlaybookRequest>): Promise<Playbook> {
    try {
      const playbook = await playbookRepository.update(id, data)
      this.store.updatePlaybook(playbook)
      return playbook
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to update playbook'
      )
      throw error
    }
  }

  async deletePlaybook(id: string): Promise<void> {
    try {
      await playbookRepository.delete(id)
      this.store.removePlaybook(id)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to delete playbook'
      )
      throw error
    }
  }

  // Runs
  async loadRuns(teamId: TeamId): Promise<void> {
    this.store.setLoading(true)
    try {
      const runs = await playbookRepository.listRuns(teamId)
      this.store.setRuns(runs)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to load runs'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  async loadRun(id: string): Promise<void> {
    this.store.setLoading(true)
    try {
      const run = await playbookRepository.getRun(id)
      this.store.setCurrentRun(run)
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to load run'
      )
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  async startRun(teamId: TeamId, request: StartRunRequest): Promise<PlaybookRun> {
    try {
      const { run } = await playbookRepository.startRun(teamId, request)
      this.store.addRun(run)
      return run
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to start run'
      )
      throw error
    }
  }

  async updateRunTask(runId: string, taskId: string, status: string): Promise<void> {
    try {
      await playbookRepository.updateRunTask(runId, taskId, status)
      // State will be updated via API response or refresh
    } catch (error) {
      this.store.setError(
        error instanceof AppError ? error.message : 'Failed to update task'
      )
      throw error
    }
  }
}

export const playbookService = new PlaybookService()
