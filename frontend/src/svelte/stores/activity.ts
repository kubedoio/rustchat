import { writable, derived, get } from 'svelte/store'
import { svelteApi } from './http'
import { authStore } from './auth'

export type ActivityType = 'mention' | 'reply' | 'reaction' | 'dm' | 'thread_reply'

export interface Activity {
    id: string
    type: ActivityType
    actorId: string
    actorUsername: string
    actorAvatarUrl?: string
    channelId: string
    channelName: string
    teamId: string
    teamName: string
    postId: string
    rootId?: string
    message?: string
    reaction?: string
    read: boolean
    createdAt: string
}

interface ActivityState {
    activities: Map<string, Activity>
    order: string[]
    unreadCount: number
    hasMore: boolean
    cursor: string | null
    filter: ActivityType | null
    isLoading: boolean
    isOpen: boolean
}

const initialState: ActivityState = {
    activities: new Map(),
    order: [],
    unreadCount: 0,
    hasMore: false,
    cursor: null,
    filter: null,
    isLoading: false,
    isOpen: false,
}

function createActivityStore() {
    const { subscribe, set, update } = writable<ActivityState>(initialState)

    function transformActivity(raw: Record<string, unknown>): Activity {
        return {
            id: String(raw.id ?? ''),
            type: (raw.type as ActivityType) ?? 'mention',
            actorId: String(raw.actor_id ?? ''),
            actorUsername: String(raw.actor_username ?? ''),
            actorAvatarUrl: raw.actor_avatar_url as string | undefined,
            channelId: String(raw.channel_id ?? ''),
            channelName: String(raw.channel_name ?? ''),
            teamId: String(raw.team_id ?? ''),
            teamName: String(raw.team_name ?? ''),
            postId: String(raw.post_id ?? ''),
            rootId: raw.root_id as string | undefined,
            message: raw.message_text as string | undefined,
            reaction: raw.reaction as string | undefined,
            read: Boolean(raw.read),
            createdAt: String(raw.created_at ?? new Date().toISOString()),
        }
    }

    async function loadActivities(refresh = false) {
        const userId = get(authStore).user?.id
        if (!userId) return

        update((state) => ({ ...state, isLoading: true }))

        try {
            const state = get({ subscribe })
            const cursor = refresh ? undefined : state.cursor ?? undefined
            const filter = state.filter

            const params = new URLSearchParams()
            if (cursor) params.set('cursor', cursor)
            params.set('limit', '50')
            if (filter) params.set('type', filter)

            const qs = params.toString()
            const path = `/users/me/activity${qs ? `?${qs}` : ''}`
            const { data } = await svelteApi.get<{
                order: string[]
                activities: Record<string, Record<string, unknown>>
                unread_count: number
                next_cursor?: string
            }>(path, { baseURL: '/api/v4' })

            const newActivities = Object.values(data.activities || {}).map((a) =>
                transformActivity(a),
            )
            const newOrder = data.order || []

            update((state) => {
                const activities = refresh ? new Map() : new Map(state.activities)
                for (const activity of newActivities) {
                    activities.set(activity.id, activity)
                }
                const order = refresh
                    ? newOrder
                    : [...state.order, ...newOrder.filter((id) => !state.order.includes(id))]
                return {
                    ...state,
                    activities,
                    order,
                    unreadCount: data.unread_count ?? 0,
                    hasMore: !!data.next_cursor,
                    cursor: data.next_cursor ?? null,
                    isLoading: false,
                }
            })
        } catch (error) {
            console.error('Failed to load activities:', error)
            update((state) => ({ ...state, isLoading: false }))
        }
    }

    async function loadMore() {
        const state = get({ subscribe })
        if (!state.hasMore || state.isLoading) return
        await loadActivities()
    }

    async function markRead(activityId: string) {
        const userId = get(authStore).user?.id
        if (!userId) return

        update((state) => {
            const activity = state.activities.get(activityId)
            if (activity && !activity.read) {
                const activities = new Map(state.activities)
                activities.set(activityId, { ...activity, read: true })
                return { ...state, activities, unreadCount: Math.max(0, state.unreadCount - 1) }
            }
            return state
        })

        try {
            await svelteApi.post(
                `/users/me/activity/read`,
                { activity_ids: [activityId] },
                { baseURL: '/api/v4' },
            )
        } catch (error) {
            console.error('Failed to mark activity as read:', error)
        }
    }

    async function markAllRead() {
        const userId = get(authStore).user?.id
        if (!userId) return

        update((state) => {
            const activities = new Map(state.activities)
            for (const activity of activities.values()) {
                activities.set(activity.id, { ...activity, read: true })
            }
            return { ...state, activities, unreadCount: 0 }
        })

        try {
            await svelteApi.post(`/users/me/activity/read-all`, {}, { baseURL: '/api/v4' })
        } catch (error) {
            console.error('Failed to mark all activities as read:', error)
        }
    }

    function setFilter(type: ActivityType | null) {
        update((state) => ({ ...state, filter: type, activities: new Map(), order: [], cursor: null }))
        void loadActivities(true)
    }

    function openFeed() {
        update((state) => ({ ...state, isOpen: true }))
        void loadActivities(true)
    }

    function closeFeed() {
        update((state) => ({ ...state, isOpen: false }))
    }

    function toggleFeed() {
        const state = get({ subscribe })
        if (state.isOpen) {
            closeFeed()
        } else {
            openFeed()
        }
    }

    return {
        subscribe,
        loadActivities,
        loadMore,
        markRead,
        markAllRead,
        setFilter,
        openFeed,
        closeFeed,
        toggleFeed,
        reset: () => set(initialState),
    }
}

export const activityStore = createActivityStore()

export const activityList = derived(activityStore, ($store) =>
    $store.order.map((id) => $store.activities.get(id)).filter((a): a is Activity => a !== undefined),
)

export const unreadActivityCount = derived(activityStore, ($store) => $store.unreadCount)
