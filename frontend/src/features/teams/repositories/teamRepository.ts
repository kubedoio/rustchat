// Team Repository - Data access for teams

import { teamsApi } from '../../../api/teams'
import type { Team, TeamMember, TeamId } from '../../../core/entities/Team'
import type { UserId } from '../../../core/entities/User'
import { withRetry } from '../../../core/services/retry'
import { isNotFoundError } from '../../../core/errors/errorUtils'

export interface CreateTeamRequest {
  name: string
  displayName: string
  description?: string
}

export const teamRepository = {
  // List user's teams
  async list(): Promise<Team[]> {
    return withRetry(async () => {
      const response = await teamsApi.list()
      return response.data.map(normalizeTeam)
    })
  },

  // List public teams available to join
  async listPublic(): Promise<Team[]> {
    return withRetry(async () => {
      const response = await teamsApi.listPublic()
      return response.data.map(normalizeTeam)
    })
  },

  // Get single team
  async getById(teamId: TeamId): Promise<Team | null> {
    return withRetry(async () => {
      try {
        const response = await teamsApi.get(teamId)
        return normalizeTeam(response.data)
      } catch (error: unknown) {
        if (isNotFoundError(error)) return null
        throw error
      }
    })
  },

  // Create new team
  async create(data: CreateTeamRequest): Promise<Team> {
    return withRetry(async () => {
      const response = await teamsApi.create({
        name: data.name,
        display_name: data.displayName,
        description: data.description,
      })
      return normalizeTeam(response.data)
    })
  },

  // Update team
  async update(teamId: TeamId, data: Partial<CreateTeamRequest>): Promise<Team> {
    return withRetry(async () => {
      const response = await teamsApi.update(teamId, {
        name: data.name,
        display_name: data.displayName,
        description: data.description,
      })
      return normalizeTeam(response.data)
    })
  },

  // Delete team
  async delete(teamId: TeamId): Promise<void> {
    await withRetry(() => teamsApi.delete(teamId))
  },

  // Join a public team
  async join(teamId: TeamId): Promise<void> {
    await withRetry(() => teamsApi.join(teamId))
  },

  // Leave a team
  async leave(teamId: TeamId): Promise<void> {
    await withRetry(() => teamsApi.leave(teamId))
  },

  // Get team members
  async getMembers(teamId: TeamId): Promise<TeamMember[]> {
    return withRetry(async () => {
      const response = await teamsApi.getMembers(teamId)
      return response.data.map(normalizeTeamMember)
    })
  },
}

function normalizeTeam(raw: unknown): Team {
  const r = raw as Record<string, unknown>
  return {
    id: r.id as TeamId,
    name: r.name as string,
    displayName: r.display_name as string,
    description: r.description as string | undefined,
    createdAt: new Date((r.created_at || Date.now()) as string | number),
    updatedAt: new Date((r.updated_at || r.created_at || Date.now()) as string | number),
    isArchived: Boolean(r.delete_at),
  }
}

function normalizeTeamMember(raw: unknown): TeamMember {
  const r = raw as Record<string, unknown>
  return {
    teamId: r.team_id as TeamId,
    userId: r.user_id as UserId,
    roles: (r.roles as string[] | undefined) || [],
    joinedAt: new Date((r.joined_at || Date.now()) as string | number),
  }
}
