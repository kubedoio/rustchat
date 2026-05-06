<script setup lang="ts">
import { useRouter, useRoute, RouterView } from 'vue-router';
import { useAuthStore } from '../../stores/auth';
import { 
    LayoutDashboard, Users, Building2, Settings, Shield, 
    Puzzle, Scale, Mail, Activity, ArrowLeft,
    KeyRound, UserPlus, BarChart3, ScrollText
} from 'lucide-vue-next';
import { computed } from 'vue';

const router = useRouter();
const route = useRoute();
const authStore = useAuthStore();
import { useConfigStore } from '../../stores/config';
const configStore = useConfigStore();

const navItems = [
    { path: '/admin', name: 'Overview', icon: LayoutDashboard, exact: true },
    { path: '/admin/users', name: 'Users', icon: Users },
    { path: '/admin/teams', name: 'Teams & Channels', icon: Building2 },
    { path: '/admin/membership-policies', name: 'Membership Policies', icon: UserPlus },
    { path: '/admin/audit-dashboard', name: 'Audit Dashboard', icon: BarChart3 },
    { path: '/admin/settings', name: 'Server Settings', icon: Settings },
    { path: '/admin/security', name: 'Security', icon: Shield },
    { path: '/admin/permissions', name: 'Permissions', icon: Shield },
    { path: '/admin/sso', name: 'SSO / OAuth', icon: KeyRound },
    { path: '/admin/integrations', name: 'Integrations', icon: Puzzle },
    { path: '/admin/compliance', name: 'Compliance', icon: Scale },
    { path: '/admin/terms', name: 'Terms of Service', icon: ScrollText },
    { path: '/admin/email', name: 'Email & SMTP', icon: Mail },
    { path: '/admin/health', name: 'System Health', icon: Activity },
];

const isActive = (item: typeof navItems[0]) => {
    if (item.exact) {
        return route.path === item.path;
    }
    return route.path.startsWith(item.path);
};

const activeItemName = computed(() => {
    const item = navItems.find(i => isActive(i));
    return item?.name || 'Admin Console';
});

const exitAdmin = () => {
    router.push('/');
};
</script>

<template>
    <div class="flex h-screen bg-bg-surface-2">
        <!-- Sidebar -->
        <aside class="w-56 bg-bg-surface-1 flex flex-col shrink-0 border-r border-border-1">
            <!-- Header -->
            <div class="h-14 flex items-center px-4 border-b border-border-1">
                <div class="flex items-center gap-2.5 min-w-0">
                    <div class="w-6 h-6 rounded-md bg-brand flex items-center justify-center shrink-0">
                        <span class="text-white text-xs font-bold">R</span>
                    </div>
                    <span class="font-semibold text-sm text-text-1 truncate">{{ configStore.siteConfig.site_name || 'RustChat' }}</span>
                    <span class="text-[10px] bg-brand/10 text-brand px-1.5 py-0.5 rounded font-medium shrink-0">Admin</span>
                </div>
            </div>

            <!-- Navigation -->
            <nav class="flex-1 overflow-y-auto py-2 px-2 space-y-0.5">
                <router-link
                    v-for="item in navItems"
                    :key="item.path"
                    :to="item.path"
                    class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-all duration-200"
                    :class="[
                        isActive(item) 
                            ? 'bg-brand/10 text-brand' 
                            : 'text-text-3 hover:bg-bg-surface-2 hover:text-text-1'
                    ]"
                >
                    <component :is="item.icon" class="w-3.5 h-3.5 shrink-0" />
                    <span class="truncate">{{ item.name }}</span>
                </router-link>
            </nav>

            <!-- Footer -->
            <div class="p-3 border-t border-border-1">
                <button
                    @click="exitAdmin"
                    class="w-full flex items-center justify-center gap-2 px-3 py-2 bg-bg-surface-2 hover:bg-border-1 rounded-lg text-xs font-medium text-text-2 transition-colors"
                >
                    <ArrowLeft class="w-3.5 h-3.5" />
                    Exit Admin Console
                </button>
                <div class="mt-2 text-[10px] text-text-4 text-center truncate">
                    Logged in as <span class="text-text-2">{{ authStore.user?.username }}</span>
                </div>
            </div>
        </aside>

        <!-- Main Content -->
        <main class="flex-1 flex flex-col min-w-0">
            <!-- Top Bar -->
            <header class="h-14 bg-bg-surface-1 border-b border-border-1 flex items-center justify-between px-6 shrink-0">
                <h1 class="text-sm font-semibold text-text-1">{{ activeItemName }}</h1>
                <div class="flex items-center gap-3">
                    <div class="w-7 h-7 rounded-full bg-brand/10 flex items-center justify-center text-brand text-xs font-bold">
                        {{ authStore.user?.username?.charAt(0).toUpperCase() }}
                    </div>
                </div>
            </header>

            <!-- Content Area -->
            <div class="flex-1 overflow-y-auto p-6">
                <div class="max-w-6xl mx-auto">
                    <RouterView />
                </div>
            </div>
        </main>
    </div>
</template>
