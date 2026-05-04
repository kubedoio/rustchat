<script lang="ts">
  import { onMount } from 'svelte'
  import type { Component } from 'svelte'
  import LoginView from './views/auth/LoginView.svelte'
  import RegisterView from './views/auth/RegisterView.svelte'
  import ForgotPasswordView from './views/auth/ForgotPasswordView.svelte'
  import ResetPasswordView from './views/auth/ResetPasswordView.svelte'
  import ChatView from './views/main/ChatView.svelte'
  import AdminConsole from './views/admin/AdminConsole.svelte'
  import { authStore, isAuthenticated } from './stores/auth'
  import { chatStore } from './stores/chat'

  type Route = {
    component: Component
    requiresAuth?: boolean
    requiresAdmin?: boolean
    params?: Record<string, string>
  }

  const staticRoutes: Record<string, Route> = {
    '/login': { component: LoginView },
    '/register': { component: RegisterView },
    '/forgot-password': { component: ForgotPasswordView },
    '/reset-password': { component: ResetPasswordView },
    '/set-password': { component: ResetPasswordView },
    '/': { component: ChatView, requiresAuth: true },
  }

  const adminRoles = new Set(['system_admin', 'org_admin', 'admin', 'administrator'])

  function matchRoute(path: string): Route {
    if (staticRoutes[path]) {
      return staticRoutes[path]
    }

    if (path === '/admin' || path.startsWith('/admin/')) {
      return { component: AdminConsole, requiresAuth: true, requiresAdmin: true }
    }

    const channelMatch = path.match(/^\/channels\/([^/]+)$/)
    if (channelMatch) {
      return { component: ChatView, requiresAuth: true, params: { id: channelMatch[1] } }
    }

    return staticRoutes['/']
  }

  let path = $state(window.location.pathname)

  const currentRoute = $derived(matchRoute(path))
  const CurrentComponent = $derived(currentRoute.component)
  const signedIn = $derived($isAuthenticated)
  const signedInUser = $derived($authStore.user)
  const signedInUserIsAdmin = $derived(adminRoles.has(signedInUser?.role || ''))

  function navigate(nextPath: string) {
    if (window.location.pathname === nextPath) {
      path = nextPath
      return
    }

    window.history.pushState({}, '', nextPath)
    path = nextPath
  }

  // Sync URL channel param into store
  $effect(() => {
    const channelId = currentRoute.params?.id
    if (channelId && $chatStore.currentChannelId !== channelId) {
      chatStore.update((state) => ({ ...state, currentChannelId: channelId }))
    }
  })

  // Sync store channel selection into URL
  $effect(() => {
    const channelId = $chatStore.currentChannelId
    if (!channelId) return

    const expectedPath = `/channels/${channelId}`
    if (path === '/') {
      navigate(expectedPath)
      return
    }
    if (path.startsWith('/channels/') && path !== expectedPath) {
      navigate(expectedPath)
    }
  })

  $effect(() => {
    if (currentRoute.requiresAuth && !signedIn) {
      navigate('/login')
      return
    }

    if (currentRoute.requiresAdmin) {
      if (!signedInUser) {
        return
      }

      if (!signedInUserIsAdmin) {
        navigate('/')
        return
      }
    }

    if ((path === '/login' || path === '/register') && signedIn) {
      const channelId = $chatStore.currentChannelId
      navigate(channelId ? `/channels/${channelId}` : '/')
    }
  })

  onMount(() => {
    const handlePopState = () => {
      path = window.location.pathname
    }

    window.addEventListener('popstate', handlePopState)

    return () => {
      window.removeEventListener('popstate', handlePopState)
    }
  })
</script>

<CurrentComponent />
