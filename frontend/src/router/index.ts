import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import client from '../api/client'

const router = createRouter({
    history: createWebHistory(),
    routes: [
        {
            path: '/login',
            name: 'login',
            component: () => import('../views/auth/LoginView.vue'),
            meta: { layout: 'auth' }
        },
        {
            path: '/register',
            name: 'register',
            component: () => import('../views/auth/RegisterView.vue'),
            meta: { layout: 'auth' }
        },
        {
            path: '/forgot-password',
            name: 'forgot-password',
            component: () => import('../views/auth/ForgotPasswordView.vue'),
            meta: { layout: 'auth' }
        },
        {
            path: '/reset-password',
            name: 'reset-password',
            component: () => import('../views/auth/ResetPasswordView.vue'),
            meta: { layout: 'auth' }
        },
        {
            path: '/set-password',
            name: 'set-password',
            component: () => import('../views/auth/ResetPasswordView.vue'),
            meta: { layout: 'auth' }
        },
        {
            path: '/',
            name: 'dashboard',
            component: () => import('../views/main/ChannelView.vue'),
            meta: { requiresAuth: true }
        },
        {
            path: '/admin',
            component: () => import('../views/admin/AdminConsole.vue'),
            meta: { requiresAuth: true, requiresAdmin: true },
            children: [
                {
                    path: '',
                    name: 'admin-dashboard',
                    component: () => import('../views/admin/AdminDashboard.vue'),
                },
                {
                    path: 'users',
                    name: 'admin-users',
                    component: () => import('../views/admin/UsersManagement.vue'),
                },
                {
                    path: 'teams',
                    name: 'admin-teams',
                    component: () => import('../views/admin/TeamsManagement.vue'),
                },
                {
                    path: 'membership-policies',
                    name: 'admin-membership-policies',
                    component: () => import('../views/admin/MembershipPolicies.vue'),
                },
                {
                    path: 'audit-dashboard',
                    name: 'admin-audit-dashboard',
                    component: () => import('../views/admin/AuditDashboard.vue'),
                },
                {
                    path: 'settings',
                    name: 'admin-settings',
                    component: () => import('../views/admin/ServerSettings.vue'),
                },
                {
                    path: 'security',
                    name: 'admin-security',
                    component: () => import('../views/admin/SecuritySettings.vue'),
                },
                {
                    path: 'permissions',
                    name: 'admin-permissions',
                    component: () => import('../views/admin/PermissionsSettings.vue'),
                },
                {
                    path: 'integrations',
                    name: 'admin-integrations',
                    component: () => import('../views/admin/IntegrationsSettings.vue'),
                },
                {
                    path: 'compliance',
                    name: 'admin-compliance',
                    component: () => import('../views/admin/ComplianceSettings.vue'),
                },
                {
                    path: 'audit',
                    name: 'admin-audit',
                    component: () => import('../views/admin/AuditLogs.vue'),
                },
                {
                    path: 'email',
                    name: 'admin-email',
                    component: () => import('../views/admin/EmailSettings.vue'),
                },
                {
                    path: 'sso',
                    name: 'admin-sso',
                    component: () => import('../views/admin/SsoSettings.vue'),
                },
                {
                    path: 'health',
                    name: 'admin-health',
                    component: () => import('../views/admin/SystemHealth.vue'),
                },
                {
                    path: 'terms',
                    name: 'admin-terms',
                    component: () => import('../views/admin/TermsSettings.vue'),
                },
            ]
        },
        {
            path: '/playbooks',
            name: 'playbooks',
            component: () => import('../views/main/PlaybooksView.vue'),
            meta: { requiresAuth: true }
        },
        {
            path: '/playbooks/new',
            name: 'playbook-create',
            component: () => import('../components/playbooks/PlaybookEditor.vue'),
            meta: { requiresAuth: true }
        },
        {
            path: '/playbooks/:id/edit',
            name: 'playbook-edit',
            component: () => import('../components/playbooks/PlaybookEditor.vue'),
            meta: { requiresAuth: true }
        },
        {
            path: '/runs/:id',
            name: 'playbook-run',
            component: () => import('../components/playbooks/PlaybookRun.vue'),
            meta: { requiresAuth: true }
        },
        {
            path: '/settings/profile',
            name: 'profile-settings',
            component: () => import('../views/settings/ProfileView.vue'),
            meta: { requiresAuth: true }
        },
        // Fallback
        { path: '/:pathMatch(.*)*', redirect: '/' }
    ]
})

router.beforeEach(async (to, _from, next) => {
    // Handle OAuth redirects before auth checks.
    const urlParams = new URLSearchParams(window.location.search)
    const code = urlParams.get('code')
    const oauth = urlParams.get('oauth')
    const auth = useAuthStore()
    if (code || oauth === '1') {
        try {
            const response = code
                ? await client.post('/oauth2/exchange', { code })
                : await client.post('/oauth2/exchange', {})
            const accessToken = response.data?.token

            if (!accessToken) {
                throw new Error('OAuth redirect did not include a usable token')
            }

            auth.token = accessToken
            await auth.fetchMe()

            urlParams.delete('code')
            urlParams.delete('oauth')
            const remaining = urlParams.toString()
            const newUrl = `${window.location.pathname}${remaining ? `?${remaining}` : ''}${window.location.hash}`
            window.history.replaceState({}, document.title, newUrl)
        } catch (error) {
            console.error('OAuth redirect handling failed', error)
            auth.token = ''
            auth.user = null
            next('/login?error=oauth_failed')
            return
        }
    }

    // Rehydrate user if token exists but user is null
    if (auth.isAuthenticated && !auth.user) {
        await auth.fetchMe()
    }

    const requiresAdmin = to.matched.some(record => record.meta.requiresAdmin)
    const isAdmin = ['system_admin', 'org_admin', 'admin', 'administrator'].includes(auth.user?.role || '')

    if (to.meta.requiresAuth && !auth.isAuthenticated) {
        next('/login')
    } else if (requiresAdmin && !isAdmin) {
        next('/') // Redirect non-admins to home
    } else if ((to.name === 'login' || to.name === 'register') && auth.isAuthenticated) {
        next('/')
    } else {
        next()
    }
})

export default router
