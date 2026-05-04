import { get, writable } from 'svelte/store'
import { authStore } from './auth'
import { svelteApi } from './http'

export type SvelteChatChannelType = 'public' | 'private' | 'direct' | 'group'

export interface SvelteChatTeam {
    id: string
    name: string
    display_name: string
}

export interface SvelteChatChannel {
    id: string
    name: string
    display_name: string
    team_id: string
    channel_type: SvelteChatChannelType
}

export interface SvelteChatFile {
    id: string
    name?: string
    url?: string
    size?: number
    mime_type?: string
    mimeType?: string
    width?: number
    height?: number
}

export interface SvelteChatPost {
    id: string
    channel_id: string
    user_id: string
    message: string
    created_at: string
    files: SvelteChatFile[]
    client_msg_id?: string
}

export interface SvelteChatMember {
    user_id: string
    username: string
    display_name?: string
    avatar_url?: string
    presence?: 'online' | 'away' | 'dnd' | 'offline'
}

export interface SvelteChatReadState {
    last_read_message_id: number | string | null
    first_unread_message_id: number | string | null
}

export interface SvelteChatState {
    teams: SvelteChatTeam[]
    channels: SvelteChatChannel[]
    currentChannelId: string | null
    messagesByChannel: Record<string, SvelteChatPost[]>
    membersByTeam: Record<string, SvelteChatMember[]>
    readStateByChannel: Record<string, SvelteChatReadState | null>
    loading: boolean
    error: string | null
}

interface PostsResponse {
    messages: unknown[]
    read_state: SvelteChatReadState | null
}

interface SendPostBody {
    channel_id: string
    message: string
    file_ids?: string[]
    client_msg_id?: string
}

const initialState: SvelteChatState = {
    teams: [],
    channels: [],
    currentChannelId: null,
    messagesByChannel: {},
    membersByTeam: {},
    readStateByChannel: {},
    loading: false,
    error: null,
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null
}

function stringField(source: Record<string, unknown>, field: string, fallback = ''): string {
    const value = source[field]

    return typeof value === 'string' ? value : fallback
}

function optionalStringField(source: Record<string, unknown>, field: string): string | undefined {
    const value = source[field]

    return typeof value === 'string' ? value : undefined
}

function normalizeChannelType(value: unknown): SvelteChatChannelType {
    return value === 'private' || value === 'direct' || value === 'group' ? value : 'public'
}

function normalizeFiles(value: unknown): SvelteChatFile[] {
    if (!Array.isArray(value)) {
        return []
    }

    return value.filter(isRecord).map((file) => ({
        id: stringField(file, 'id'),
        name: optionalStringField(file, 'name'),
        url: optionalStringField(file, 'url'),
        size: typeof file.size === 'number' ? file.size : undefined,
        mime_type: optionalStringField(file, 'mime_type'),
        mimeType: optionalStringField(file, 'mimeType'),
        width: typeof file.width === 'number' ? file.width : undefined,
        height: typeof file.height === 'number' ? file.height : undefined,
    }))
}

function normalizeTeam(value: unknown): SvelteChatTeam {
    const team = isRecord(value) ? value : {}
    const name = stringField(team, 'name')

    return {
        id: stringField(team, 'id'),
        name,
        display_name: stringField(team, 'display_name', name),
    }
}

function normalizeChannel(value: unknown): SvelteChatChannel {
    const channel = isRecord(value) ? value : {}
    const name = stringField(channel, 'name')

    return {
        id: stringField(channel, 'id'),
        name,
        display_name: stringField(channel, 'display_name', name),
        team_id: stringField(channel, 'team_id'),
        channel_type: normalizeChannelType(channel.channel_type ?? channel.type),
    }
}

function normalizePost(value: unknown, fallbackChannelId: string): SvelteChatPost {
    const post = isRecord(value) ? value : {}

    return {
        id: stringField(post, 'id'),
        channel_id: stringField(post, 'channel_id', fallbackChannelId),
        user_id: stringField(post, 'user_id'),
        message: stringField(post, 'message'),
        created_at: stringField(post, 'created_at', new Date().toISOString()),
        files: normalizeFiles(post.files),
        client_msg_id: optionalStringField(post, 'client_msg_id'),
    }
}

function normalizeMember(value: unknown): SvelteChatMember {
    const member = isRecord(value) ? value : {}

    return {
        user_id: stringField(member, 'user_id', stringField(member, 'id')),
        username: stringField(member, 'username'),
        display_name: optionalStringField(member, 'display_name'),
        avatar_url: optionalStringField(member, 'avatar_url'),
        presence:
            member.presence === 'online' || member.presence === 'away' || member.presence === 'dnd' || member.presence === 'offline'
                ? member.presence
                : undefined,
    }
}

function errorMessage(error: unknown, fallback: string): string {
    return error instanceof Error ? error.message : fallback
}

function createChatStore() {
    const { subscribe, set, update } = writable<SvelteChatState>(initialState)

    async function fetchTeams(): Promise<SvelteChatTeam[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<unknown[]>('/teams')
            const teams = data.map(normalizeTeam)
            update((state) => ({ ...state, teams, loading: false }))

            return teams
        } catch (error) {
            update((state) => ({ ...state, loading: false, error: errorMessage(error, 'Failed to fetch teams') }))
            throw error
        }
    }

    async function fetchChannels(): Promise<SvelteChatChannel[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<unknown[]>('/channels')
            const channels = data.map(normalizeChannel)
            update((state) => {
                const currentChannelId = state.currentChannelId ?? channels[0]?.id ?? null

                return { ...state, channels, currentChannelId, loading: false }
            })

            return channels
        } catch (error) {
            update((state) => ({ ...state, loading: false, error: errorMessage(error, 'Failed to fetch channels') }))
            throw error
        }
    }

    async function fetchMessages(channelId: string): Promise<SvelteChatPost[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<PostsResponse>(`/channels/${channelId}/posts`)
            const messages = data.messages.map((post) => normalizePost(post, channelId))
            update((state) => ({
                ...state,
                messagesByChannel: { ...state.messagesByChannel, [channelId]: messages },
                readStateByChannel: { ...state.readStateByChannel, [channelId]: data.read_state },
                loading: false,
            }))

            return messages
        } catch (error) {
            update((state) => ({ ...state, loading: false, error: errorMessage(error, 'Failed to fetch messages') }))
            throw error
        }
    }

    async function fetchMembers(teamId: string): Promise<SvelteChatMember[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<unknown[]>(`/teams/${teamId}/members`)
            const members = data.map(normalizeMember)
            update((state) => ({
                ...state,
                membersByTeam: { ...state.membersByTeam, [teamId]: members },
                loading: false,
            }))

            return members
        } catch (error) {
            update((state) => ({ ...state, loading: false, error: errorMessage(error, 'Failed to fetch members') }))
            throw error
        }
    }

    async function selectChannel(channelId: string): Promise<void> {
        update((state) => ({ ...state, currentChannelId: channelId, error: null }))

        const channel = get(chatStore).channels.find((candidate) => candidate.id === channelId)
        if (channel?.team_id && !get(chatStore).membersByTeam[channel.team_id]) {
            await fetchMembers(channel.team_id)
        }

        if (!get(chatStore).messagesByChannel[channelId]) {
            await fetchMessages(channelId)
        }
    }

    async function bootstrap(): Promise<void> {
        await Promise.all([fetchTeams(), fetchChannels()])

        const channelId = get(chatStore).currentChannelId
        if (channelId) {
            const channel = get(chatStore).channels.find((candidate) => candidate.id === channelId)
            if (channel?.team_id) {
                await fetchMembers(channel.team_id)
            }
            await fetchMessages(channelId)
        }
    }

    async function sendMessage(channelId: string, message: string, file_ids?: string[]): Promise<SvelteChatPost> {
        const body: SendPostBody = { channel_id: channelId, message }

        if (file_ids && file_ids.length > 0) {
            body.file_ids = file_ids
        }

        try {
            const { data } = await svelteApi.post<unknown>(`/channels/${channelId}/posts`, body)
            const post = normalizePost(data, channelId)
            update((state) => ({
                ...state,
                messagesByChannel: {
                    ...state.messagesByChannel,
                    [channelId]: [...(state.messagesByChannel[channelId] ?? []), post],
                },
                error: null,
            }))

            return post
        } catch (error) {
            update((state) => ({ ...state, error: errorMessage(error, 'Failed to send message') }))
            throw error
        }
    }

    function addLocalFileMessage(channelId: string, files: SvelteChatFile[], message = ''): SvelteChatPost {
        const user = get(authStore).user
        const now = new Date().toISOString()
        const post: SvelteChatPost = {
            id: `local-${Date.now()}`,
            channel_id: channelId,
            user_id: user?.id ?? '',
            message,
            created_at: now,
            files,
            client_msg_id: `local-${now}`,
        }

        update((state) => ({
            ...state,
            messagesByChannel: {
                ...state.messagesByChannel,
                [channelId]: [...(state.messagesByChannel[channelId] ?? []), post],
            },
        }))

        return post
    }

    return {
        subscribe,
        bootstrap,
        selectChannel,
        fetchTeams,
        fetchChannels,
        fetchMessages,
        sendMessage,
        fetchMembers,
        addLocalFileMessage,
        reset: () => set(initialState),
    }
}

export const chatStore = createChatStore()
