import api from '../../../api/client'
import { postsApi, type Post } from '../../../api/posts'
import { AppError } from '../../../core/errors/AppError'

export interface ThreadQueryParams {
  cursor?: string
  limit?: number
}

export interface ThreadResponse {
  order: string[]
  posts: Record<string, Post>
  next_cursor?: string
}

export const threadService = {
  /**
   * Fetch a thread with parent post and replies
   */
  async getThread(postId: string, params: ThreadQueryParams = {}): Promise<ThreadResponse> {
    try {
      const query = new URLSearchParams()
      if (params.cursor) query.set('cursor', params.cursor)
      if (params.limit) query.set('limit', params.limit.toString())

      const queryString = query.toString()
      const url = `/posts/${postId}/thread${queryString ? `?${queryString}` : ''}`

      const response = await api.get<ThreadResponse>(url)
      return response.data
    } catch (error) {
      throw new AppError(
        error instanceof Error ? error.message : 'Failed to fetch thread',
        'NETWORK_ERROR',
        true
      )
    }
  },

  /**
   * Send a reply to a thread
   */
  async sendReply(
    channelId: string,
    rootId: string,
    message: string,
    fileIds: string[] = []
  ): Promise<Post> {
    try {
      const response = await api.post<Post>('/posts', {
        channel_id: channelId,
        root_id: rootId,
        parent_id: rootId,
        message,
        file_ids: fileIds,
      })
      return response.data
    } catch (error) {
      throw new AppError(
        error instanceof Error ? error.message : 'Failed to send reply',
        'NETWORK_ERROR',
        true
      )
    }
  },

  /**
   * Get thread using postsApi (alternative method)
   * Returns array of posts instead of ThreadResponse format
   */
  async getThreadPosts(postId: string): Promise<Post[]> {
    try {
      const response = await postsApi.getThread(postId)
      return response.data.order
        .map(id => response.data.posts[id])
        .filter((post): post is Post => post !== undefined)
    } catch (error) {
      throw new AppError(
        error instanceof Error ? error.message : 'Failed to fetch thread posts',
        'NETWORK_ERROR',
        true
      )
    }
  },
}
