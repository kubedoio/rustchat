import { useThreadStore } from '../stores/threadStore'
import type { Post } from '@/api/posts'

export function registerThreadHandlers() {
  const threadStore = useThreadStore()

  return {
    handleNewPost(post: Post) {
      // If this is a reply to the currently open thread
      if (
        post.root_post_id &&
        post.root_post_id !== post.id &&
        post.root_post_id === threadStore.parentPostId
      ) {
        threadStore.onNewReply(post)
      }
    },

    handlePostDeleted(postId: string) {
      threadStore.onPostDeleted(postId)
    },

    handlePostUpdated(post: Post) {
      if (post.root_post_id === threadStore.parentPostId) {
        // Update reply in thread if present
        const index = threadStore.replies.findIndex(r => r.id === post.id)
        if (index !== -1) {
          threadStore.replies[index] = post
        }
      }
    },
  }
}
