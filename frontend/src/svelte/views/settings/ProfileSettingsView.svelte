<script lang="ts">
  import { ArrowLeft, User } from 'lucide-svelte'
  import ProfileTab from '../../components/settings/tabs/ProfileTab.svelte'

  function navigate(path: string) {
    window.history.pushState({}, '', path)
    window.dispatchEvent(new PopStateEvent('popstate'))
  }
</script>

<main class="min-h-screen bg-bg-app text-text-1">
  <header class="border-b border-border-1 bg-bg-surface-1 px-6 py-4">
    <div class="mx-auto flex max-w-4xl items-center gap-3">
      <button
        type="button"
        class="flex h-9 w-9 items-center justify-center rounded-r-2 text-text-3 transition-standard hover:bg-bg-surface-2 hover:text-text-1"
        aria-label="Back to chat"
        onclick={() => navigate('/')}
      >
        <ArrowLeft class="h-5 w-5" />
      </button>
      <div class="flex h-10 w-10 items-center justify-center rounded-r-2 bg-brand/10 text-brand">
        <User class="h-5 w-5" />
      </div>
      <div>
        <h1 class="text-xl font-bold">Profile Settings</h1>
        <p class="text-sm text-text-3">Update your visible identity and teammate context.</p>
      </div>
    </div>
  </header>

  <section class="mx-auto max-w-4xl px-6 py-6">
    <ProfileTab />
  </section>
</main>
