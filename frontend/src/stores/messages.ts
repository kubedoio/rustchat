import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { postsApi, type Post } from '../api/posts'
import { useChannelStore } from './channels'
import { useAuthStore } from './auth'

export interface Message {
    id: string
    channelId: string
    userId: string
    username: string
    avatarUrl?: string
    email?: string
    content: string
    timestamp: string
    reactions: { emoji: string; count: number; users: string[] }[]
    threadCount?: number
    lastReplyAt?: string
    rootId?: string
    files?: { id: string; name: string; url: string; size: number; mime_type: string }[]
    isPinned: boolean
    isSaved: boolean
    status?: 'sending' | 'delivered' | 'failed'
    clientMsgId?: string
}

function postToMessage(post: Post): Message {
    return {
        id: post.id,
        channelId: post.channel_id,
        userId: post.user_id,
        username: post.username || 'Unknown',
        avatarUrl: post.avatar_url,
        email: post.email,
        content: post.message,
        timestamp: post.created_at,
        reactions: post.reactions?.map((r: any) => ({
            emoji: r.emoji,
            count: r.count,
            users: r.users.map((u: any) => u.toString())
        })) || [],
        rootId: post.root_post_id,
        threadCount: post.reply_count || 0,
        lastReplyAt: post.last_reply_at,
        files: post.files || [],
        isPinned: post.is_pinned,
        isSaved: post.is_saved || false,
        status: 'delivered',
        clientMsgId: (post as any).client_msg_id,
    }
}

export const useMessageStore = defineStore('messages', () => {
    // Messages grouped by channel
    const messagesByChannel = ref<Record<string, Message[]>>({})
    const repliesByThread = ref<Record<string, Message[]>>({})
    const hasMoreOlderByChannel = ref<Record<string, boolean>>({}) // Track if we can load more history
    const loading = ref(false)
    const error = ref<string | null>(null)

    function getMessages(channelId: string) {
        return computed(() => messagesByChannel.value[channelId] || [])
    }

    const hasMoreOlder = computed(() => (channelId: string) => hasMoreOlderByChannel.value[channelId] ?? true)

    async function fetchMessages(channelId: string) {
        loading.value = true
        error.value = null
        try {
            const response = await postsApi.list(channelId, { limit: 50 })
            const messages = response.data
                .filter(p => !p.root_post_id)
                .map(postToMessage)
                .reverse()
            messagesByChannel.value[channelId] = messages

            // If we got fewer than 50, we probably reached the end
            hasMoreOlderByChannel.value[channelId] = response.data.length >= 50
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to fetch messages'
        } finally {
            loading.value = false
        }
    }

    async function fetchOlderMessages(channelId: string) {
        if (loading.value || !hasMoreOlder.value(channelId)) return

        const currentMessages = messagesByChannel.value[channelId] || []
        if (currentMessages.length === 0) {
            await fetchMessages(channelId)
            return
        }

        // Use the timestamp of the OLDEST message as the cursor
        const before = currentMessages[0]!.timestamp

        loading.value = true
        try {
            const response = await postsApi.list(channelId, { before, limit: 50 })
            const olderMessages = response.data
                .filter(p => !p.root_post_id)
                .map(postToMessage)
                .reverse()

            if (olderMessages.length > 0) {
                messagesByChannel.value[channelId] = [...olderMessages, ...currentMessages]
            }

            hasMoreOlderByChannel.value[channelId] = response.data.length >= 50
        } catch (e: any) {
            console.error('Failed to fetch older messages:', e)
        } finally {
            loading.value = false
        }
    }

    async function fetchThread(rootId: string) {
        loading.value = true
        error.value = null
        try {
            const response = await postsApi.getThread(rootId)
            const replies = response.data.map(postToMessage)
            repliesByThread.value[rootId] = replies
        } catch (e: any) {
            error.value = e.response?.data?.message || 'Failed to fetch thread'
        } finally {
            loading.value = false
        }
    }

    // Optimized for WebSocket usage
    function addOptimisticMessage(message: Message) {
        if (!messagesByChannel.value[message.channelId]) {
            messagesByChannel.value[message.channelId] = []
        }
        messagesByChannel.value[message.channelId]?.push(message)
    }

    function updateOptimisticMessage(clientMsgId: string, serverMsg: Message) {
        if (!messagesByChannel.value[serverMsg.channelId]) return

        const channelMessages = messagesByChannel.value[serverMsg.channelId]
        if (!channelMessages) return;

        const index = channelMessages.findIndex(m => m.clientMsgId === clientMsgId)
        if (index !== -1) {
            // Update in place
            channelMessages[index] = serverMsg
        } else {
            // Fallback: append if not found (should be rare/race)
            channelMessages.push(serverMsg)
        }
    }

    function handleNewMessage(post: Post) {
        const message = postToMessage(post)

        if (!messagesByChannel.value[message.channelId]) {
            messagesByChannel.value[message.channelId] = []
        }

        const channelMessages = messagesByChannel.value[message.channelId]
        if (!channelMessages) return

        // Check if message already exists (avoid duplicates) or reconcile with optimist
        const existingIndex = channelMessages.findIndex(m => {
            const idMatch = m.id === message.id
            const clientMatch = m.clientMsgId && m.clientMsgId === message.clientMsgId
            return idMatch || clientMatch
        })

        if (existingIndex !== -1) {
            // Update existing in main feed
            channelMessages[existingIndex] = message
        } else if (!message.rootId) {
            // Only add to main feed if it's a root message
            channelMessages.push(message)
        }

        // Also add to repliesByThread if it's a reply and that thread is being viewed/cached
        if (message.rootId) {
            if (!repliesByThread.value[message.rootId]) {
                repliesByThread.value[message.rootId] = []
            }

            const threadReplies = repliesByThread.value[message.rootId]
            if (threadReplies && !threadReplies.some(r => r.id === message.id)) {
                threadReplies.push(message)
            }
        }

        // Handle notifications
        const channelStore = useChannelStore()
        const authStore = useAuthStore()

        // Increment unread count if not current channel
        if (channelStore.currentChannelId !== message.channelId) {
            channelStore.incrementUnread(message.channelId)

            // Check for mention
            const currentUser = authStore.user
            if (currentUser && message.content.includes(`@${currentUser.username}`)) {
                channelStore.incrementMention(message.channelId)
            }
        }
    }

    function clearMessages(channelId?: string) {
        if (channelId) {
            delete messagesByChannel.value[channelId]
        } else {
            messagesByChannel.value = {}
        }
    }

    async function pinMessage(messageId: string, channelId: string) {
        try {
            await postsApi.pin(messageId)
            // Update local state
            const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
            if (message) {
                message.isPinned = true
            }
        } catch (e: any) {
            error.value = 'Failed to pin message'
            throw e
        }
    }

    async function unpinMessage(messageId: string, channelId: string) {
        try {
            await postsApi.unpin(messageId)
            // Update local state
            const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
            if (message) {
                message.isPinned = false
            }
        } catch (e: any) {
            error.value = 'Failed to unpin message'
            throw e
        }
    }

    async function saveMessage(messageId: string, channelId: string) {
        try {
            await postsApi.save(messageId)
            // Update local state
            const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
            if (message) {
                message.isSaved = true
            }
        } catch (e: any) {
            error.value = 'Failed to save message'
            throw e
        }
    }

    async function unsaveMessage(messageId: string, channelId: string) {
        try {
            await postsApi.unsave(messageId)
            // Update local state
            const message = messagesByChannel.value[channelId]?.find(m => m.id === messageId)
            if (message) {
                message.isSaved = false
            }
        } catch (e: any) {
            error.value = 'Failed to unsave message'
            throw e
        }
    }

    async function searchMessages(channelId: string, query: string) {
        loading.value = true
        error.value = null
        try {
            const response = await postsApi.list(channelId, { q: query })
            return response.data.map(postToMessage)
        } catch (e: any) {
            error.value = 'Failed to search messages'
            throw e
        } finally {
            loading.value = false
        }
    }

    async function fetchPinnedMessages(channelId: string) {
        loading.value = true
        error.value = null
        try {
            const response = await postsApi.list(channelId, { is_pinned: true })
            return response.data.map(postToMessage)
        } catch (e: any) {
            error.value = 'Failed to fetch pinned messages'
            throw e
        } finally {
            loading.value = false
        }
    }

    async function fetchSavedMessages() {
        loading.value = true
        try {
            const response = await postsApi.getSaved()
            return response.data.map(postToMessage)
        } catch (e: any) {
            error.value = 'Failed to fetch saved messages'
            throw e
        } finally {
            loading.value = false
        }
    }

    function handleMessageUpdate(data: any) {
        if (!data.id) return

        // 1. Update in main channels
        for (const cid in messagesByChannel.value) {
            const messages = messagesByChannel.value[cid]
            if (!messages) continue

            const index = messages.findIndex(m => m.id === data.id)
            if (index !== -1) {
                const msg = messages[index]
                if (!msg) continue

                if (data.message !== undefined) msg.content = data.message
                if (data.is_pinned !== undefined) msg.isPinned = data.is_pinned
                if (data.reply_count !== undefined) msg.threadCount = data.reply_count
                if (data.reply_count_inc) {
                    msg.threadCount = (msg.threadCount || 0) + data.reply_count_inc
                }
            }
        }

        // 2. Update in cached threads
        for (const rootId in repliesByThread.value) {
            const replies = repliesByThread.value[rootId]
            if (!replies) continue

            const index = replies.findIndex(m => m.id === data.id)
            if (index !== -1) {
                const msg = replies[index]
                if (!msg) continue

                if (data.message !== undefined) msg.content = data.message
                if (data.is_pinned !== undefined) msg.isPinned = data.is_pinned
            }
        }
    }

    function handleMessageDelete(messageId: string) {
        // 1. Remove from main channels
        for (const cid in messagesByChannel.value) {
            const messages = messagesByChannel.value[cid]
            if (messages) {
                const index = messages.findIndex(m => m.id === messageId)
                if (index !== -1) {
                    messages.splice(index, 1)
                }
            }
        }

        // 2. Remove from cached threads
        for (const rootId in repliesByThread.value) {
            const replies = repliesByThread.value[rootId]
            if (replies) {
                const index = replies.findIndex(m => m.id === messageId)
                if (index !== -1) {
                    replies.splice(index, 1)
                }
            }
        }
    }

    function handleReactionAdded(data: any) {
        // data: { post_id, user_id, emoji_name, created_at, ... }
        // We need to find the post. It usually comes via broadcast which has channel_id in envelope context, 
        // but robustly the event data should contain channel_id OR we search? 
        // The previous implementation relied on context. 
        // Let's assume we can pass channelId to this function or it's in data.
        // My backend broadcast for reaction added DOES include channel_id in the *envelope*, but data itself is just Reaction struct.
        // Reaction struct doesn't have channelId. 
        // However, I updated `add_reaction` in backend to broadcast the reaction... wait. 
        // The `ws` handler passes `envelope.data`. If `envelope.channel_id` is set, `useWebSocket` could pass it.
        // `useWebSocket` calls `handleReactionAdded(envelope.data)`.
        // If data doesn't have channel_id, we are stuck unless we search.
        // Simplification: Search all channels or require channel_id.
        // I should have injected channel_id into the data payload or use envelope context.

        // Quick fix: loop through loaded channels to find post.
        for (const cid in messagesByChannel.value) {
            const messages = messagesByChannel.value[cid];
            if (!messages) continue;

            const msg = messages.find(m => m.id === data.post_id)
            if (msg) {
                // Found message
                const existingReaction = msg.reactions.find(r => r.emoji === data.emoji_name)
                if (existingReaction) {
                    if (!existingReaction.users.includes(data.user_id)) {
                        existingReaction.users.push(data.user_id)
                        existingReaction.count++
                    }
                } else {
                    msg.reactions.push({
                        emoji: data.emoji_name,
                        count: 1,
                        users: [data.user_id]
                    })
                }
                return
            }
        }
    }

    function handleReactionRemoved(data: any) {
        for (const cid in messagesByChannel.value) {
            const messages = messagesByChannel.value[cid];
            if (!messages) continue;

            const msg = messages.find(m => m.id === data.post_id)
            if (msg) {
                const index = msg.reactions.findIndex(r => r.emoji === data.emoji_name)
                if (index !== -1) {
                    const reaction = msg.reactions[index]
                    if (!reaction) continue;

                    const userIndex = reaction.users.indexOf(data.user_id)
                    if (userIndex !== -1) {
                        reaction.users.splice(userIndex, 1)
                        reaction.count--
                        if (reaction.count <= 0) {
                            msg.reactions.splice(index, 1)
                        }
                    }
                }
                return
            }
        }
    }

    return {
        messagesByChannel,
        repliesByThread,
        loading,
        error,
        getMessages,
        fetchMessages,
        fetchThread,
        addOptimisticMessage,
        updateOptimisticMessage,
        handleNewMessage,
        handleMessageUpdate,
        handleMessageDelete,
        handleReactionAdded,
        handleReactionRemoved,
        clearMessages,
        pinMessage,
        unpinMessage,
        saveMessage,
        unsaveMessage,
        fetchSavedMessages,
        fetchPinnedMessages,
        searchMessages,
        hasMoreOlder,
        fetchOlderMessages,
    }
})
