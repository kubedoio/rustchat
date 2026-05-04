import { writable } from 'svelte/store'
import { svelteApi } from './http'

export interface TeamMember {
    user_id: string
    username: string
    display_name?: string
    avatar_url?: string
    role?: string
    presence?: 'online' | 'away' | 'dnd' | 'offline'
}

export interface TeamState {
    membersByTeam: Record<string, TeamMember[]>
    loading: boolean
    error: string | null
}

const initialState: TeamState = {
    membersByTeam: {},
    loading: false,
    error: null,
}

function createTeamStore() {
    const { subscribe, set, update } = writable<TeamState>(initialState)

    async function fetchMembers(teamId: string): Promise<TeamMember[]> {
        update((state) => ({ ...state, loading: true, error: null }))
        try {
            const { data } = await svelteApi.get<unknown[]>(`/teams/${teamId}/members`)
            const members: TeamMember[] = data.map((item: unknown) => {
                const m =
                    typeof item === 'object' && item !== null
                        ? (item as Record<string, unknown>)
                        : {}
                const presence =
                    m.presence === 'online' ||
                    m.presence === 'away' ||
                    m.presence === 'dnd' ||
                    m.presence === 'offline'
                        ? m.presence
                        : undefined
                return {
                    user_id: String(m.user_id ?? m.id ?? ''),
                    username: String(m.username ?? ''),
                    display_name: typeof m.display_name === 'string' ? m.display_name : undefined,
                    avatar_url: typeof m.avatar_url === 'string' ? m.avatar_url : undefined,
                    role: typeof m.role === 'string' ? m.role : undefined,
                    presence: presence as TeamMember['presence'],
                }
            })
            update((state) => ({
                ...state,
                membersByTeam: { ...state.membersByTeam, [teamId]: members },
                loading: false,
            }))
            return members
        } catch (error) {
            update((state) => ({
                ...state,
                loading: false,
                error: error instanceof Error ? error.message : 'Failed to fetch team members',
            }))
            throw error
        }
    }

    async function leaveTeam(teamId: string): Promise<void> {
        await svelteApi.post(`/teams/${teamId}/leave`)
        update((state) => {
            const next = { ...state.membersByTeam }
            delete next[teamId]
            return { ...state, membersByTeam: next }
        })
    }

    return {
        subscribe,
        fetchMembers,
        leaveTeam,
        reset: () => set(initialState),
    }
}

export const teamStore = createTeamStore()
