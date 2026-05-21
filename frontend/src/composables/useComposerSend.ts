import { type Ref, type ComputedRef } from 'vue'
import { useToast } from './useToast'
import { useChannelStore } from '@/features/channels/stores/channelStore'
import { useCallsStore } from '../stores/calls'
import { getErrorMessage } from '@/core/errors/errorUtils'

export interface SendPayload {
  content: string
  file_ids: string[]
}

interface UseComposerSendOptions {
  content: Ref<string>
  attachedFiles: Ref<{ file: File; uploading: boolean; progress: number; uploaded?: { id: string } }[]>
  canSend: ComputedRef<boolean>
  onClearDraft: () => void
  onResetComposer: () => void
  emitSend: (payload: SendPayload) => void
}

export function useComposerSend(options: UseComposerSendOptions) {
  const toast = useToast()
  const channelStore = useChannelStore()
  const callsStore = useCallsStore()

  async function handleCommandAction(command: string, args: string[]): Promise<boolean> {
    const channelId = channelStore.currentChannelId
    if (!channelId) return false

    switch (command) {
      case 'call': {
        const subCommand = args[0]

        if (subCommand === 'start' || !subCommand) {
          const existingCall = callsStore.currentChannelCall(channelId)
          if (existingCall) {
            toast.error('A call is already in progress', 'Join the existing call instead')
            return true
          }

          try {
            await callsStore.startCall(channelId)
            toast.success('Call started', 'You are now in a call')
          } catch (error: unknown) {
            toast.error('Failed to start call', getErrorMessage(error, 'Unknown error'))
          }
          return true
        }

        if (subCommand === 'join') {
          try {
            await callsStore.joinCall(channelId)
          } catch (error: unknown) {
            toast.error('Failed to join call', getErrorMessage(error, 'Unknown error'))
          }
          return true
        }

        if (subCommand === 'leave') {
          if (callsStore.isInCall) {
            await callsStore.leaveCall()
          } else {
            toast.error('Not in a call', 'You are not currently in a call')
          }
          return true
        }

        if (subCommand === 'end') {
          if (callsStore.isInCall) {
            await callsStore.endCall()
          } else {
            toast.error('Not in a call', 'You are not currently in a call')
          }
          return true
        }

        return false
      }
      default:
        return false
    }
  }

  async function handleSend() {
    if (!options.canSend.value) return

    const trimmedText = options.content.value.trim()
    const commandMatch = trimmedText.match(/^\^k\s*(.*)$/i)
    if (commandMatch) {
      const commandPayload = (commandMatch[1] ?? '').trim()
      if (!commandPayload) {
        toast.error('No command selected', 'Use Ctrl/Cmd+K or type ^k and choose a command')
        return
      }

      const parts = commandPayload.split(/\s+/)
      const command = (parts[0] ?? '').toLowerCase()
      const args = parts.slice(1)
      const handled = await handleCommandAction(command, args)
      if (handled) {
        options.onClearDraft()
        options.onResetComposer()
        return
      }

      toast.error('Unknown command', `The command "${commandPayload}" is not available`)
      return
    }

    const fileIds = options.attachedFiles.value
      .filter((attachment) => !attachment.uploading && attachment.uploaded)
      .map((attachment) => attachment.uploaded!.id)

    options.emitSend({
      content: options.content.value,
      file_ids: fileIds,
    })

    options.onClearDraft()
    options.onResetComposer()
  }

  return { handleSend, handleCommandAction }
}
