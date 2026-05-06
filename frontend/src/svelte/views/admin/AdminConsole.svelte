<script lang="ts">
  import { onMount } from 'svelte'
  import { ArrowLeft } from 'lucide-svelte'
  import { authStore } from '../../stores/auth'
  import { configStore } from '../../stores/config'
  import { adminRoutes, findAdminRoute } from './adminRoutes'

  let path = $state(typeof window === 'undefined' ? '/admin' : window.location.pathname)

  const activeRoute = $derived(findAdminRoute(path))
  const ActiveComponent = $derived(activeRoute.component)

  function isActive(route: (typeof adminRoutes)[number]) {
    return route.exact ? path === route.path : path.startsWith(route.path)
  }

  function navigate(nextPath: string) {
    if (typeof window === 'undefined') {
      return
    }

    if (window.location.pathname !== nextPath) {
      window.history.pushState({}, '', nextPath)
    }
    path = nextPath
    window.dispatchEvent(new PopStateEvent('popstate'))
  }

  function initials(value: string | null | undefined) {
    return (value || 'A').slice(0, 1).toUpperCase()
  }

  onMount(() => {
    const handlePopState = () => {
      path = window.location.pathname
    }

    window.addEventListener('popstate', handlePopState)
    return () => window.removeEventListener('popstate', handlePopState)
  })
</script>

<div class="flex h-screen bg-bg-surface-2 text-text-1" data-testid="admin-console">
  <aside class="flex w-64 shrink-0 flex-col border-r border-border-1 bg-bg-surface-1" aria-label="Admin console">
    <div class="flex h-14 items-center border-b border-border-1 px-4">
      <div class="flex min-w-0 items-center gap-2.5">
        <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-r-2 bg-brand text-xs font-bold text-brand-foreground">
          R
        </div>
        <span class="truncate text-sm font-semibold text-text-1">{$configStore.siteConfig.site_name || 'RustChat'}</span>
        <span class="shrink-0 rounded bg-brand/10 px-1.5 py-0.5 text-[10px] font-semibold text-brand">Admin</span>
      </div>
    </div>

    <nav class="flex-1 space-y-0.5 overflow-y-auto px-2 py-2" aria-label="Admin sections">
      {#each adminRoutes as route (route.path)}
        {@const Icon = route.icon}
        <button
          type="button"
          data-testid="admin-nav-link"
          class="flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left text-xs font-semibold transition-standard {isActive(route) ? 'bg-brand/10 text-brand' : 'text-text-3 hover:bg-bg-surface-2 hover:text-text-1'}"
          aria-current={isActive(route) ? 'page' : undefined}
          onclick={() => navigate(route.path)}
        >
          <Icon class="h-3.5 w-3.5 shrink-0" />
          <span class="truncate">{route.label}</span>
        </button>
      {/each}
    </nav>

    <div class="border-t border-border-1 p-3">
      <button
        type="button"
        class="flex w-full items-center justify-center gap-2 rounded-lg bg-bg-surface-2 px-3 py-2 text-xs font-semibold text-text-2 transition-standard hover:bg-border-1"
        onclick={() => navigate('/')}
      >
        <ArrowLeft class="h-3.5 w-3.5" />
        Exit Admin Console
      </button>
      <div class="mt-2 truncate text-center text-[10px] text-text-4">
        Logged in as <span class="text-text-2">{$authStore.user?.username || 'admin'}</span>
      </div>
    </div>
  </aside>

  <main class="flex min-w-0 flex-1 flex-col">
    <header class="flex h-14 shrink-0 items-center justify-between border-b border-border-1 bg-bg-surface-1 px-6">
      <h1 class="text-sm font-semibold text-text-1">{activeRoute.label}</h1>
      <div class="flex h-8 w-8 items-center justify-center rounded-full bg-brand/10 text-xs font-bold text-brand">
        {initials($authStore.user?.display_name || $authStore.user?.username)}
      </div>
    </header>

    <div class="flex-1 overflow-y-auto p-6">
      <div class="mx-auto max-w-6xl">
        <ActiveComponent label={activeRoute.label} eyebrow={activeRoute.eyebrow} description={activeRoute.description} />
      </div>
    </div>
  </main>
</div>
