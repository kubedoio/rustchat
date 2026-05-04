<script lang="ts">
  import { onMount } from 'svelte'
  import type { Component } from 'svelte'
  import LoginView from './views/auth/LoginView.svelte'
  import RegisterView from './views/auth/RegisterView.svelte'
  import ForgotPasswordView from './views/auth/ForgotPasswordView.svelte'
  import ResetPasswordView from './views/auth/ResetPasswordView.svelte'
  import ChatView from './views/main/ChatView.svelte'
  import { isAuthenticated } from './stores/auth'

  type Route = {
    component: Component
    requiresAuth?: boolean
  }

  const routes: Record<string, Route> = {
    '/login': { component: LoginView },
    '/register': { component: RegisterView },
    '/forgot-password': { component: ForgotPasswordView },
    '/reset-password': { component: ResetPasswordView },
    '/set-password': { component: ResetPasswordView },
    '/': { component: ChatView, requiresAuth: true },
  }

  let path = $state(window.location.pathname)

  const currentRoute = $derived(routes[path] ?? routes['/'])
  const CurrentComponent = $derived(currentRoute.component)
  const signedIn = $derived($isAuthenticated)

  function navigate(nextPath: string) {
    if (window.location.pathname === nextPath) {
      path = nextPath
      return
    }

    window.history.pushState({}, '', nextPath)
    path = nextPath
  }

  $effect(() => {
    if (currentRoute.requiresAuth && !signedIn) {
      navigate('/login')
      return
    }

    if ((path === '/login' || path === '/register') && signedIn) {
      navigate('/')
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
