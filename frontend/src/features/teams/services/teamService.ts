// Team Service - Business logic for teams

import { teamRepository, type CreateTeamRequest } from '../repositories/teamRepository'
import type { Team, TeamId } from '../../../core/entities/Team'
import { useTeamStore } from '../stores/teamStore'
import { AppError } from '../../../core/errors/AppError'

const ACTIVE_TEAM_KEY = 'active_team_id'

class TeamService {
  private get store() {
    return useTeamStore()
  }

  // Load teams and auto-select
  async loadTeams(): Promise<void> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      const teams = await teamRepository.list()
      this.store.setTeams(teams)

      // Auto-select first team if none selected
      const firstTeam = teams[0]
      if (!this.store.currentTeamId && firstTeam) {
        this.selectTeam(firstTeam.id)
      }
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to fetch teams')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Load public teams
  async loadPublicTeams(): Promise<void> {
    this.store.setLoading(true)
    try {
      const teams = await teamRepository.listPublic()
      this.store.setPublicTeams(teams)
    } finally {
      this.store.setLoading(false)
    }
  }

  // Select a team (persists to localStorage)
  selectTeam(teamId: TeamId): void {
    this.store.setCurrentTeamId(teamId)
    this.saveActiveTeamId(teamId)
  }

  // Create team
  async createTeam(data: CreateTeamRequest): Promise<Team> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      const team = await teamRepository.create(data)
      this.store.addTeam(team)
      this.selectTeam(team.id)
      return team
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to create team')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Join a public team
  async joinTeam(teamId: TeamId): Promise<void> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      await teamRepository.join(teamId)
      // Refresh teams to include the joined one
      await this.loadTeams()
      // Select the joined team
      this.selectTeam(teamId)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to join team')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Leave a team
  async leaveTeam(teamId: TeamId): Promise<void> {
    this.store.setLoading(true)
    this.store.clearError()

    try {
      await teamRepository.leave(teamId)
      this.store.removeTeam(teamId)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to leave team')
      throw error
    } finally {
      this.store.setLoading(false)
    }
  }

  // Update team
  async updateTeam(teamId: TeamId, data: Partial<CreateTeamRequest>): Promise<Team> {
    try {
      const team = await teamRepository.update(teamId, data)
      this.store.updateTeam(team)
      return team
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to update team')
      throw error
    }
  }

  // Delete team
  async deleteTeam(teamId: TeamId): Promise<void> {
    try {
      await teamRepository.delete(teamId)
      this.store.removeTeam(teamId)
    } catch (error) {
      this.store.setError(error instanceof AppError ? error.message : 'Failed to delete team')
      throw error
    }
  }

  // Load team members
  async loadMembers(teamId: TeamId): Promise<void> {
    this.store.setLoading(true)
    try {
      const members = await teamRepository.getMembers(teamId)
      this.store.setMembers(members)
    } finally {
      this.store.setLoading(false)
    }
  }

  // WebSocket event handlers
  handleTeamCreated(team: Team): void {
    this.store.addTeam(team)
  }

  handleTeamUpdated(team: Team): void {
    this.store.updateTeam(team)
  }

  handleTeamDeleted(teamId: TeamId): void {
    this.store.removeTeam(teamId)
  }

  // Initialize from storage (call on app startup)
  initialize(): void {
    const savedId = this.getActiveTeamId()
    if (savedId) {
      this.store.setCurrentTeamId(savedId)
    }
  }

  // Private helpers
  private getActiveTeamId(): TeamId | null {
    try {
      return localStorage.getItem(ACTIVE_TEAM_KEY) as TeamId | null
    } catch {
      return null
    }
  }

  private saveActiveTeamId(teamId: TeamId): void {
    try {
      localStorage.setItem(ACTIVE_TEAM_KEY, teamId)
    } catch {
      // Ignore storage errors
    }
  }
}

export const teamService = new TeamService()
