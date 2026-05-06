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
    unreadCount?: number
    mentionCount?: number
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

export interface SvelteChatReaction {
    emoji: string
    count: number
    users: string[]
}

export interface SvelteChatPost {
    id: string
    channel_id: string
    user_id: string
    message: string
    created_at: string
    files: SvelteChatFile[]
    client_msg_id?: string
    username?: string
    avatar_url?: string
    reactions?: SvelteChatReaction[]
    reply_count?: number
    root_id?: string
    parent_id?: string
}

export interface SvelteChatMember {
    user_id: string
    username: string
    display_name?: string
    avatar_url?: string
    presence?: 'online' | 'away' | 'dnd' | 'offline'
    status_text?: string | null
    status_emoji?: string | null
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
    unreadCounts: Record<string, number>
    threadsByParent: Record<string, SvelteChatPost[]>
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
    unreadCounts: {},
    threadsByParent: {},
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

function customStatusField(source: Record<string, unknown>, field: 'text' | 'emoji'): string | undefined {
    const value = source.custom_status

    if (!isRecord(value)) {
        return undefined
    }

    return optionalStringField(value, field)
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

function normalizeReactions(value: unknown): SvelteChatReaction[] | undefined {
    if (!Array.isArray(value)) {
        return undefined
    }

    const reactions = value.filter(isRecord).map((r) => ({
        emoji: stringField(r, 'emoji'),
        count: typeof r.count === 'number' ? r.count : 0,
        users: Array.isArray(r.users) ? r.users.filter((u: unknown): u is string => typeof u === 'string') : [],
    }))

    return reactions.length > 0 ? reactions : undefined
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
        unreadCount: typeof channel.unreadCount === 'number'
            ? channel.unreadCount
            : typeof channel.unread_count === 'number'
                ? channel.unread_count
                : undefined,
        mentionCount: typeof channel.mentionCount === 'number'
            ? channel.mentionCount
            : typeof channel.mention_count === 'number'
                ? channel.mention_count
                : undefined,
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
        username: optionalStringField(post, 'username'),
        avatar_url: optionalStringField(post, 'avatar_url'),
        reactions: normalizeReactions(post.reactions),
        reply_count: typeof post.reply_count === 'number' ? post.reply_count : undefined,
        root_id: optionalStringField(post, 'root_id'),
        parent_id: optionalStringField(post, 'parent_id'),
    }
}

function normalizeMember(value: unknown): SvelteChatMember {
    const member = isRecord(value) ? value : {}

    return {
        user_id: stringField(member, 'user_id', stringField(member, 'id')),
        username: stringField(member, 'username'),
        display_name: optionalStringField(member, 'display_name'),
        avatar_url: optionalStringField(member, 'avatar_url'),
        status_text: optionalStringField(member, 'status_text') ?? customStatusField(member, 'text'),
        status_emoji: optionalStringField(member, 'status_emoji') ?? customStatusField(member, 'emoji'),
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

    async function fetchChannels(teamId?: string): Promise<SvelteChatChannel[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const params = teamId ? { team_id: teamId } : undefined
            const { data } = await svelteApi.get<unknown[]>('/channels', { params })
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
                messagesByChannel: {
                    ...state.messagesByChannel,
                    [channelId]: mergeMessages(messages, state.messagesByChannel[channelId] ?? []),
                },
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

    async function fetchUnreadCounts(): Promise<void> {
        try {
            const { data } = await svelteApi.get<{
                channels: Array<{ channel_id: string; unread_count: number; mention_count?: number }>
                teams?: Array<{ team_id: string; unread_count: number }>
            }>('/unreads/overview')
            const unreadCounts: Record<string, number> = {}
            data.channels.forEach((c) => {
                unreadCounts[c.channel_id] = c.unread_count
            })
            update((state) => ({
                ...state,
                channels: state.channels.map((channel) => ({
                    ...channel,
                    unreadCount: unreadCounts[channel.id] ?? channel.unreadCount,
                })),
                unreadCounts,
            }))
        } catch (error) {
            console.error('Failed to fetch unread counts:', error)
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
        const teams = await fetchTeams()
        const teamId = teams[0]?.id
        if (teamId) {
            await fetchChannels(teamId)
        }

        const channelId = get(chatStore).currentChannelId
        if (channelId) {
            const channel = get(chatStore).channels.find((candidate) => candidate.id === channelId)
            if (channel?.team_id) {
                await fetchMembers(channel.team_id)
            }
            await Promise.all([fetchMessages(channelId), fetchUnreadCounts()])
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

    function addMessage(channelId: string, post: SvelteChatPost): void {
        update((state) => ({
            ...state,
            messagesByChannel: {
                ...state.messagesByChannel,
                [channelId]: [...(state.messagesByChannel[channelId] ?? []), post],
            },
        }))
    }

    function updateMessage(channelId: string, post: Partial<SvelteChatPost> & { id: string }): void {
        update((state) => {
            const messages = state.messagesByChannel[channelId] ?? []
            const updated = messages.map((msg) =>
                msg.id === post.id ? { ...msg, ...post } : msg,
            )
            return {
                ...state,
                messagesByChannel: { ...state.messagesByChannel, [channelId]: updated },
            }
        })
    }

    function deleteMessage(channelId: string, postId: string): void {
        update((state) => {
            const messages = state.messagesByChannel[channelId] ?? []
            const filtered = messages.filter((msg) => msg.id !== postId)
            return {
                ...state,
                messagesByChannel: { ...state.messagesByChannel, [channelId]: filtered },
            }
        })
    }

    function updateMemberPresence(userId: string, presence: string): void {
        update((state) => {
            const nextMembersByTeam: Record<string, SvelteChatMember[]> = {}
            for (const [teamId, members] of Object.entries(state.membersByTeam)) {
                nextMembersByTeam[teamId] = members.map((member) =>
                    member.user_id === userId
                        ? {
                              ...member,
                              presence:
                                  presence === 'online' || presence === 'away' || presence === 'dnd' || presence === 'offline'
                                      ? presence
                                      : member.presence,
                          }
                        : member,
                )
            }
            return { ...state, membersByTeam: nextMembersByTeam }
        })
    }

    async function fetchThreadReplies(threadId: string): Promise<SvelteChatPost[]> {
        update((state) => ({ ...state, loading: true, error: null }))

        try {
            const { data } = await svelteApi.get<{
                order: string[]
                posts: Record<string, unknown>
                next_cursor?: string
            }>(`/posts/${threadId}/thread`)

            const allPosts = data.order
                .map((id) => (isRecord(data.posts[id]) ? data.posts[id] : null))
                .filter((post): post is Record<string, unknown> => post !== null)

            const channelId = allPosts[0] ? stringField(allPosts[0], 'channel_id') : ''
            const replies = allPosts
                .filter((post) => stringField(post, 'id') !== threadId)
                .map((post) => normalizePost(post, channelId))

            update((state) => ({
                ...state,
                threadsByParent: { ...state.threadsByParent, [threadId]: replies },
                loading: false,
            }))

            return replies
        } catch (error) {
            update((state) => ({
                ...state,
                loading: false,
                error: errorMessage(error, 'Failed to fetch thread replies'),
            }))
            throw error
        }
    }

    async function sendThreadReply(threadId: string, channelId: string, message: string): Promise<SvelteChatPost> {
        try {
            const { data } = await svelteApi.post<unknown>('/posts', {
                channel_id: channelId,
                root_id: threadId,
                parent_id: threadId,
                message,
                file_ids: [],
            })
            const post = normalizePost(data, channelId)
            update((state) => {
                const messages = state.messagesByChannel[channelId] ?? []
                const updatedMessages = messages.map((msg) =>
                    msg.id === threadId ? { ...msg, reply_count: (msg.reply_count ?? 0) + 1 } : msg,
                )
                return {
                    ...state,
                    messagesByChannel: { ...state.messagesByChannel, [channelId]: updatedMessages },
                    threadsByParent: {
                        ...state.threadsByParent,
                        [threadId]: [...(state.threadsByParent[threadId] ?? []), post],
                    },
                    error: null,
                }
            })

            return post
        } catch (error) {
            update((state) => ({ ...state, error: errorMessage(error, 'Failed to send thread reply') }))
            throw error
        }
    }

    async function uploadFile(file: File): Promise<SvelteChatFile> {
        const formData = new FormData()
        formData.append('file', file)
        const state = get(chatStore)
        if (state.currentChannelId) {
            formData.append('channel_id', state.currentChannelId)
        }

        const { data } = await svelteApi.postFormData<unknown>('/files', formData)
        const fileRecord = isRecord(data) ? data : {}

        return {
            id: stringField(fileRecord, 'id'),
            name: optionalStringField(fileRecord, 'name') ?? file.name,
            url: optionalStringField(fileRecord, 'url'),
            size: typeof fileRecord.size === 'number' ? fileRecord.size : file.size,
            mime_type: optionalStringField(fileRecord, 'mime_type') ?? file.type,
            mimeType: optionalStringField(fileRecord, 'mimeType') ?? file.type,
            width: typeof fileRecord.width === 'number' ? fileRecord.width : undefined,
            height: typeof fileRecord.height === 'number' ? fileRecord.height : undefined,
        }
    }

    function updatePostField(postId: string, field: string, value: unknown): void {
        const channelId = get(chatStore).currentChannelId
        if (!channelId) return
        chatStore.update((state) => {
            const messages = state.messagesByChannel[channelId] ?? []
            const msg = messages.find((m) => m.id === postId)
            if (msg) {
                ;(msg as unknown as Record<string, unknown>)[field] = value
            }
            return state
        })
    }

    async function addReaction(postId: string, emojiName: string): Promise<void> {
        await svelteApi.post(`/posts/${postId}/reactions`, { emoji_name: emojiName })
        const channelId = get(chatStore).currentChannelId
        if (!channelId) return
        chatStore.update((state) => {
            const messages = state.messagesByChannel[channelId] ?? []
            const msg = messages.find((m) => m.id === postId)
            if (!msg) return state
            const existing = msg.reactions?.find((r) => r.emoji === emojiName)
            const userId = get(authStore).user?.id
            if (existing && userId) {
                existing.count += 1
                existing.users = [...existing.users, userId]
            } else if (userId) {
                msg.reactions = [...(msg.reactions ?? []), { emoji: emojiName, count: 1, users: [userId] }]
            }
            return state
        })
    }

    async function removeReaction(postId: string, emojiName: string): Promise<void> {
        await svelteApi.delete_(`/posts/${postId}/reactions/${encodeURIComponent(emojiName)}`)
        const channelId = get(chatStore).currentChannelId
        if (!channelId) return
        chatStore.update((state) => {
            const messages = state.messagesByChannel[channelId] ?? []
            const msg = messages.find((m) => m.id === postId)
            if (!msg) return state
            const userId = get(authStore).user?.id
            const reaction = msg.reactions?.find((r) => r.emoji === emojiName)
            if (reaction && userId) {
                reaction.count -= 1
                reaction.users = reaction.users.filter((id) => id !== userId)
                if (reaction.count <= 0) {
                    msg.reactions = msg.reactions?.filter((r) => r.emoji !== emojiName)
                }
            }
            return state
        })
    }

    async function savePost(postId: string): Promise<void> {
        await svelteApi.post(`/posts/${postId}/save`)
        updatePostField(postId, 'isSaved', true)
    }

    async function unsavePost(postId: string): Promise<void> {
        await svelteApi.delete_(`/posts/${postId}/save`)
        updatePostField(postId, 'isSaved', false)
    }

    async function pinPost(postId: string): Promise<void> {
        await svelteApi.post(`/posts/${postId}/pin`)
        updatePostField(postId, 'isPinned', true)
    }

    async function unpinPost(postId: string): Promise<void> {
        await svelteApi.delete_(`/posts/${postId}/pin`)
        updatePostField(postId, 'isPinned', false)
    }

    async function markPostUnread(postId: string): Promise<void> {
        try {
            await svelteApi.post(`/posts/${postId}/set_unread`)
        } catch {
            const userId = get(authStore).user?.id
            if (userId) {
                await svelteApi.post(`/users/${userId}/posts/${postId}/set_unread`)
            }
        }
    }

    return {
        subscribe,
        update,
        bootstrap,
        selectChannel,
        fetchTeams,
        fetchChannels,
        fetchMessages,
        sendMessage,
        fetchMembers,
        fetchUnreadCounts,
        addLocalFileMessage,
        addMessage,
        updateMessage,
        deleteMessage,
        updateMemberPresence,
        fetchThreadReplies,
        sendThreadReply,
        uploadFile,
        addReaction,
        removeReaction,
        savePost,
        unsavePost,
        pinPost,
        unpinPost,
        markPostUnread,
        reset: () => set(initialState),
    }
}

function mergeMessages(fetched: SvelteChatPost[], existing: SvelteChatPost[]): SvelteChatPost[] {
    const merged = new Map<string, SvelteChatPost>()

    for (const message of fetched) {
        merged.set(message.id, message)
    }
    for (const message of existing) {
        merged.set(message.id, message)
    }

    return Array.from(merged.values()).sort((a, b) => {
        const left = new Date(a.created_at).getTime()
        const right = new Date(b.created_at).getTime()
        return left - right
    })
}

export const chatStore = createChatStore()
