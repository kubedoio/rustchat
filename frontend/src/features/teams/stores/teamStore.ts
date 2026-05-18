// Team Store - Compatible with legacy stores/teams.ts API

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useStorage } from '@vueuse/core'
import { teamsApi, type CreateTeamRequest } from '../../../api/teams'
import type { TeamId } from '../../../core/entities/Team'
import { getApiErrorMessage } from '../../../core/errors/errorUtils'

export const useTeamStore = defineStore('teamStore', () => {
  // Internal Map for feature architecture compatibility
  const _teamsMap = ref<Map<TeamId, any>>(new Map())
  const publicTeams = ref<any[]>([])
  const members = ref<any[]>([])
  const currentTeamId = useStorage<TeamId | null>('active_team_id', null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Legacy-compatible computed: teams as Array
  const teams = computed(() => Array.from(_teamsMap.value.values()))

  const currentTeam = computed(() => {
    if (!currentTeamId.value) return null
    return _teamsMap.value.get(currentTeamId.value) || null
  })

  // Actions - Simple state mutations (feature architecture)
  function setTeams(items: any[]) {
    _teamsMap.value.clear()
    for (const team of items) {
      _teamsMap.value.set(team.id, team)
    }
  }

  function addTeam(team: any) {
    _teamsMap.value.set(team.id, team)
  }

  function updateTeam(team: any) {
    const existing = _teamsMap.value.get(team.id)
    if (existing) {
      _teamsMap.value.set(team.id, { ...existing, ...team })
    }
  }

  function removeTeam(teamId: TeamId) {
    _teamsMap.value.delete(teamId)

    // If we removed the current team, select another
    if (currentTeamId.value === teamId) {
      const remaining = teams.value
      currentTeamId.value = remaining[0]?.id || null
    }
  }

  function setPublicTeams(items: any[]) {
    publicTeams.value = items
  }

  function setCurrentTeamId(teamId: TeamId | null) {
    currentTeamId.value = teamId
  }

  function setMembers(items: any[]) {
    members.value = items
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

  function clear() {
    _teamsMap.value.clear()
    publicTeams.value = []
    members.value = []
    currentTeamId.value = null
  }

  // Legacy async methods
  async function fetchTeams() {
    loading.value = true
    error.value = null
    try {
      const response = await teamsApi.list()
      _teamsMap.value.clear()
      for (const team of response.data) {
        _teamsMap.value.set(team.id, team)
      }
      // Auto-select first team if none selected
      if (!currentTeamId.value && response.data.length > 0) {
        currentTeamId.value = response.data[0]?.id || null
      }
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to fetch teams'
    } finally {
      loading.value = false
    }
  }

  async function fetchPublicTeams() {
    loading.value = true
    error.value = null
    try {
      const response = await teamsApi.listPublic()
      publicTeams.value = response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to fetch public teams'
    } finally {
      loading.value = false
    }
  }

  async function joinTeam(teamId: string) {
    loading.value = true
    error.value = null
    try {
      await teamsApi.join(teamId)
      // Refresh user's teams
      await fetchTeams()
      // Select the joined team
      currentTeamId.value = teamId
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to join team'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function leaveTeam(teamId: string) {
    loading.value = true
    error.value = null
    try {
      await teamsApi.leave(teamId)
      // Remove from local teams list
      removeTeam(teamId)
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to leave team'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function createTeam(data: CreateTeamRequest) {
    loading.value = true
    error.value = null
    try {
      const response = await teamsApi.create(data)
      addTeam(response.data)
      return response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to create team'
      throw e
    } finally {
      loading.value = false
    }
  }

  function selectTeam(teamId: string) {
    currentTeamId.value = teamId
  }

  async function fetchMembers(teamId: string) {
    loading.value = true
    error.value = null
    try {
      const response = await teamsApi.getMembers(teamId)
      members.value = response.data
    } catch (e: unknown) {
      error.value = getApiErrorMessage(e) || 'Failed to fetch members'
    } finally {
      loading.value = false
    }
  }

  return {
    // State
    teams,
    publicTeams,
    members,
    currentTeamId,
    currentTeam,
    loading,
    error,

    // Feature actions
    setTeams,
    addTeam,
    updateTeam,
    setPublicTeams,
    setCurrentTeamId,
    setMembers,
    setLoading,
    setError,
    clearError,
    clear,

    // Legacy actions
    fetchTeams,
    fetchPublicTeams,
    joinTeam,
    leaveTeam,
    createTeam,
    selectTeam,
    fetchMembers,
    removeTeam
  }
})
