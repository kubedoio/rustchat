<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { FileText, CheckCircle, AlertTriangle, ExternalLink } from 'lucide-svelte'

  interface Props {
    open?: boolean
    termsUrl?: string
    termsText?: string
    submitting?: boolean
    error?: string
    onAccept?: () => void | Promise<void>
    onDecline?: () => void
  }

  let {
    open = false,
    termsUrl = '',
    termsText = '',
    submitting = false,
    error = '',
    onAccept,
    onDecline,
  }: Props = $props()

  const dispatch = createEventDispatcher<{
    accept: void
    decline: void
  }>()

  let accepted = $state(false)
  let internalSubmitting = $state(false)
  let internalError = $state('')

  const isSubmitting = $derived(submitting || internalSubmitting)
  const displayError = $derived(error || internalError)

  $effect(() => {
    if (open) {
      accepted = false
      internalSubmitting = false
      internalError = ''
    }
  })

  function getErrorMessage(value: unknown): string {
    return value instanceof Error ? value.message : 'Unable to accept terms. Please try again.'
  }

  async function handleAccept() {
    if (!accepted || isSubmitting) return
    internalError = ''

    if (onAccept) {
      internalSubmitting = true
      try {
        await onAccept()
        dispatch('accept')
      } catch (err) {
        internalError = getErrorMessage(err)
      } finally {
        internalSubmitting = false
      }
      return
    }

    dispatch('accept')
  }

  function handleDecline() {
    if (isSubmitting) return
    onDecline?.()
    dispatch('decline')
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm"
    data-testid="terms-acceptance-modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="terms-title"
  >
    <div
      class="relative bg-bg-surface-1 rounded-xl shadow-2xl border border-border-1 w-full max-w-2xl max-h-[90vh] flex flex-col"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.key === 'Escape' && e.preventDefault()}
      role="document"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="flex items-start gap-3 px-6 py-5 border-b border-border-1">
        <div class="rounded-lg bg-brand/10 p-2 shrink-0">
          <FileText class="w-5 h-5 text-brand" />
        </div>
        <div class="flex-1 min-w-0">
          <h2 id="terms-title" class="text-lg font-semibold text-text-1">Terms of Service</h2>
          <p class="text-xs text-text-3 mt-1">
            Please review and accept the terms to continue using RustChat.
          </p>
        </div>
        <div class="px-2 py-1 bg-warning/10 text-warning text-[10px] rounded font-medium shrink-0">
          Required
        </div>
      </div>

      <!-- Error Alert -->
      {#if displayError}
        <div class="mx-6 mt-4 flex items-center gap-2 p-3 bg-danger/10 border border-danger/20 rounded-lg text-danger text-xs">
          <AlertTriangle class="w-4 h-4 shrink-0" />
          {displayError}
        </div>
      {/if}

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if termsUrl}
          <div class="p-3 bg-bg-surface-2 rounded-lg mb-4">
            <a
              href={termsUrl}
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1.5 text-sm text-brand hover:underline"
            >
              <ExternalLink class="w-4 h-4" aria-hidden="true" />
              View Terms of Service
            </a>
          </div>
        {/if}

        {#if termsText}
          <div class="prose prose-sm max-w-none text-text-1">
            <pre class="whitespace-pre-wrap font-sans text-sm leading-relaxed">{termsText}</pre>
          </div>
        {:else if !termsUrl}
          <p class="text-sm text-text-3">
            By continuing to use this application, you agree to abide by all applicable terms,
            policies, and guidelines.
          </p>
        {/if}
      </div>

      <!-- Footer -->
      <div class="px-6 py-5 border-t border-border-1 space-y-4">
        <label class="flex items-start gap-3 cursor-pointer">
          <input
            bind:checked={accepted}
            type="checkbox"
            disabled={isSubmitting}
            class="w-4 h-4 text-brand rounded border-border-1 mt-0.5 shrink-0"
          />
          <span class="text-xs text-text-2">
            I have read and agree to the Terms of Service. I understand that by accepting these
            terms, I am bound by the policies and guidelines outlined above.
          </span>
        </label>

        <div class="flex items-center justify-end gap-3">
          <button
            type="button"
            onclick={handleDecline}
            disabled={isSubmitting}
            class="px-4 py-2 text-text-3 hover:text-text-1 text-xs font-medium transition-colors"
          >
            Decline
          </button>
          <button
            type="button"
            onclick={handleAccept}
            disabled={!accepted || isSubmitting}
            class="flex items-center gap-2 px-4 py-2 bg-brand hover:bg-brand/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-xs font-medium transition-colors"
          >
            <CheckCircle class="w-3.5 h-3.5" />
            {isSubmitting ? 'Accepting...' : 'Accept Terms'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
