import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

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
                    path: 'health',
                    name: 'admin-health',
                    component: () => import('../views/admin/SystemHealth.vue'),
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
    const auth = useAuthStore()

    // Rehydrate user if token exists but user is null
    if (auth.isAuthenticated && !auth.user) {
        await auth.fetchMe()
    }

    if (to.meta.requiresAuth && !auth.isAuthenticated) {
        next('/login')
    } else if (to.meta.requiresAdmin && auth.user?.role !== 'system_admin' && auth.user?.role !== 'org_admin') {
        next('/') // Redirect non-admins to home
    } else if ((to.name === 'login' || to.name === 'register') && auth.isAuthenticated) {
        next('/')
    } else {
        next()
    }
})

export default router
