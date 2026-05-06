import type { Component } from 'svelte'
import {
    Activity,
    BarChart3,
    Building2,
    KeyRound,
    LayoutDashboard,
    Mail,
    Puzzle,
    Scale,
    ScrollText,
    ServerCog,
    Shield,
    UserPlus,
    Users,
} from 'lucide-svelte'
import AdminDashboard from './AdminDashboard.svelte'
import AdminAuditLogs from './AdminAuditLogs.svelte'
import AdminComplianceSettings from './AdminComplianceSettings.svelte'
import AdminEmailSettings from './AdminEmailSettings.svelte'
import AdminHealth from './AdminHealth.svelte'
import AdminIntegrationsSettings from './AdminIntegrationsSettings.svelte'
import AdminMembershipAuditDashboard from './AdminMembershipAuditDashboard.svelte'
import AdminMembershipPolicies from './AdminMembershipPolicies.svelte'
import AdminPermissions from './AdminPermissions.svelte'
import AdminSecuritySettings from './AdminSecuritySettings.svelte'
import AdminServerSettings from './AdminServerSettings.svelte'
import AdminSsoSettings from './AdminSsoSettings.svelte'
import AdminTeams from './AdminTeams.svelte'
import AdminTermsSettings from './AdminTermsSettings.svelte'
import AdminUsers from './AdminUsers.svelte'

export interface AdminRoute {
    path: string
    label: string
    icon: any
    component: Component
    exact?: boolean
    eyebrow?: string
    description?: string
}

export const adminRoutes: AdminRoute[] = [
    { path: '/admin', label: 'Overview', icon: LayoutDashboard, component: AdminDashboard, exact: true },
    { path: '/admin/users', label: 'Users', icon: Users, component: AdminUsers },
    { path: '/admin/teams', label: 'Teams & Channels', icon: Building2, component: AdminTeams },
    {
        path: '/admin/membership-policies',
        label: 'Membership Policies',
        icon: UserPlus,
        component: AdminMembershipPolicies,
    },
    {
        path: '/admin/audit-dashboard',
        label: 'Audit Dashboard',
        icon: BarChart3,
        component: AdminMembershipAuditDashboard,
    },
    {
        path: '/admin/settings',
        label: 'Server Settings',
        icon: ServerCog,
        component: AdminServerSettings,
    },
    {
        path: '/admin/security',
        label: 'Security',
        icon: Shield,
        component: AdminSecuritySettings,
    },
    { path: '/admin/permissions', label: 'Permissions', icon: Shield, component: AdminPermissions },
    { path: '/admin/sso', label: 'SSO / OAuth', icon: KeyRound, component: AdminSsoSettings },
    {
        path: '/admin/integrations',
        label: 'Integrations',
        icon: Puzzle,
        component: AdminIntegrationsSettings,
    },
    {
        path: '/admin/compliance',
        label: 'Compliance',
        icon: Scale,
        component: AdminComplianceSettings,
    },
    { path: '/admin/audit', label: 'Audit Logs', icon: BarChart3, component: AdminAuditLogs },
    { path: '/admin/email', label: 'Email & SMTP', icon: Mail, component: AdminEmailSettings },
    { path: '/admin/health', label: 'System Health', icon: Activity, component: AdminHealth },
    {
        path: '/admin/terms',
        label: 'Terms of Service',
        icon: ScrollText,
        component: AdminTermsSettings,
    },
]

export function findAdminRoute(path: string): AdminRoute {
    return adminRoutes.find((route) => (route.exact ? path === route.path : path.startsWith(route.path))) ?? adminRoutes[0]
}
