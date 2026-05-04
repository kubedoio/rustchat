<script lang="ts">
  import { authStore } from '../stores/auth'

  async function logout() {
    await authStore.logout()
  }
</script>

<main class="min-h-screen bg-bg-app text-text-1">
  <section class="mx-auto flex min-h-screen max-w-5xl flex-col justify-center px-6 py-12">
    <div class="rounded-r-3 border border-border-1 bg-bg-surface-1 p-8 shadow-2">
      <p class="text-sm font-semibold uppercase tracking-[0.18em] text-brand">Svelte migration</p>
      <h1 class="mt-3 text-3xl font-semibold text-text-1">RustChat Svelte shell is live</h1>
      <p class="mt-4 max-w-2xl text-base text-text-2">
        This is the first migration checkpoint. Auth routes and the application boot path now run through
        Svelte while the high-risk chat, settings, calls, admin, and playbook flows are migrated behind
        parity tests.
      </p>

      {#if $authStore.user}
        <p class="mt-6 rounded-r-2 bg-bg-surface-2 px-4 py-3 text-sm text-text-2">
          Signed in as <strong class="text-text-1">{$authStore.user.email ?? $authStore.user.username}</strong>.
        </p>
      {/if}

      <div class="mt-8 flex flex-wrap gap-3">
        <button
          type="button"
          class="rounded-r-1 bg-brand px-4 py-2 text-sm font-medium text-brand-foreground transition-standard hover:bg-brand-hover"
          on:click={logout}
        >
          Sign out
        </button>
        <a class="rounded-r-1 border border-border-1 px-4 py-2 text-sm font-medium text-text-2 hover:text-text-1" href="/login">
          Auth screen
        </a>
      </div>
    </div>
  </section>
</main>
