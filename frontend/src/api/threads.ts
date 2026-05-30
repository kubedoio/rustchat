import api from './client'

export interface Thread {
  id: string
  reply_count: number
  last_reply_at: number
  last_viewed_at: number
  participants: string[]
  post: {
    id: string
    channel_id: string
    user_id: string
    message: string
    create_at: number
  }
  unread_replies: number
  unread_mentions: number
  is_following?: boolean
}

export interface ThreadResponse {
  threads: Thread[]
  total: number
  total_unread_threads: number
  total_unread_mentions: number
}

export const threadsApi = {
  /**
   * Mark a thread as read up to a specific timestamp.
   * Uses current timestamp if not specified.
   */
  markAsRead: (threadId: string, teamId: string, timestamp?: number) => {
    const ts = timestamp || Date.now()
    return api.put(`/users/me/teams/${teamId}/threads/${threadId}/read/${ts}`, {})
  },

  /**
   * Mark all threads in a team as read
   */
  markAllAsRead: (teamId: string) => api.put(`/users/me/teams/${teamId}/threads/read`, {}),

  /**
   * Mark a thread as unread from a specific post
   */
  markAsUnread: (threadId: string, teamId: string, postId: string) =>
    api.post(`/users/me/teams/${teamId}/threads/${threadId}/set_unread/${postId}`, {}),

  /**
   * Follow a thread
   */
  follow: (threadId: string, teamId: string) =>
    api.put(`/users/me/teams/${teamId}/threads/${threadId}/following`, {}),

  /**
   * Unfollow a thread
   */
  unfollow: (threadId: string, teamId: string) =>
    api.delete(`/users/me/teams/${teamId}/threads/${threadId}/following`),

  /**
   * Get threads for current user in a team
   */
  list: (
    teamId: string,
    params?: {
      page?: number
      per_page?: number
      unread?: boolean
      since?: number
    }
  ) => api.get<ThreadResponse>(`/users/me/teams/${teamId}/threads`, { params }),

  /**
   * Get a single thread
   */
  get: (threadId: string, teamId: string) =>
    api.get<Thread>(`/users/me/teams/${teamId}/threads/${threadId}`),
}
