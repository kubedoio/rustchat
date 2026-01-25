import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useStorage } from '@vueuse/core'
import { teamsApi, type Team, type CreateTeamRequest, type TeamMember } from '../api/teams'

export const useTeamStore = defineStore('teams', () => {
    const teams = ref<Team[]>([])
    const publicTeams = ref<Team[]>([])
    const members = ref<TeamMember[]>([])
    const currentTeamId = useStorage<string | null>('active_team_id', null)
    const loading = ref(false)
    const error = ref<string | null>(null)

    const currentTeam = computed(() =>
        teams.value.find(t => t.id === currentTeamId.value) || null
    )

    async function fetchTeams() {
        loading.value = true
        error.value = null
        try {
            const response = await teamsApi.list()
            teams.value = response.data
            // Auto-select first team if none selected
            if (!currentTeamId.value && teams.value.length > 0) {
                currentTeamId.value = teams.value[0]?.id || null
            }
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to fetch teams'
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
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to fetch public teams'
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
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to join team'
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
            teams.value = teams.value.filter(t => t.id !== teamId)
            if (currentTeamId.value === teamId) {
                currentTeamId.value = teams.value[0]?.id || null
            }
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to leave team'
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
            teams.value.push(response.data)
            return response.data
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to create team'
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
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to fetch members'
        } finally {
            loading.value = false
        }
    }

    function updateTeam(updated: Team) {
        const index = teams.value.findIndex(t => t.id === updated.id)
        if (index !== -1) {
            teams.value[index] = updated
        }
    }

    function removeTeam(teamId: string) {
        teams.value = teams.value.filter(t => t.id !== teamId)
        if (currentTeamId.value === teamId) {
            currentTeamId.value = teams.value[0]?.id || null
        }
    }

    return {
        teams,
        publicTeams,
        members,
        currentTeamId,
        currentTeam,
        loading,
        error,
        fetchTeams,
        fetchPublicTeams,
        joinTeam,
        leaveTeam,
        createTeam,
        selectTeam,
        updateTeam,
        removeTeam,
        fetchMembers,
    }
})

