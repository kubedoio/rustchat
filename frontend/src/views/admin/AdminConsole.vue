<script setup lang="ts">
import { useRouter, useRoute, RouterView } from 'vue-router';
import { useAuthStore } from '../../stores/auth';
import { 
    LayoutDashboard, Users, Building2, Settings, Shield, 
    Puzzle, Scale, FileText, Mail, Activity, ArrowLeft
} from 'lucide-vue-next';

const router = useRouter();
const route = useRoute();
const authStore = useAuthStore();
import { useConfigStore } from '../../stores/config';
const configStore = useConfigStore();

const navItems = [
    { path: '/admin', name: 'Overview', icon: LayoutDashboard, exact: true },
    { path: '/admin/users', name: 'Users', icon: Users },
    { path: '/admin/teams', name: 'Teams & Channels', icon: Building2 },
    { path: '/admin/settings', name: 'Server Settings', icon: Settings },
    { path: '/admin/security', name: 'Security', icon: Shield },
    { path: '/admin/integrations', name: 'Integrations', icon: Puzzle },
    { path: '/admin/compliance', name: 'Compliance', icon: Scale },
    { path: '/admin/audit', name: 'Audit Logs', icon: FileText },
    { path: '/admin/email', name: 'Email & SMTP', icon: Mail },
    { path: '/admin/health', name: 'System Health', icon: Activity },
];

const isActive = (item: typeof navItems[0]) => {
    if (item.exact) {
        return route.path === item.path;
    }
    return route.path.startsWith(item.path);
};

const exitAdmin = () => {
    router.push('/');
};
</script>

<template>
    <div class="flex h-screen bg-gray-100 dark:bg-gray-950">
        <!-- Sidebar -->
        <aside class="w-64 bg-gray-900 text-white flex flex-col shrink-0">
            <!-- Header -->
            <div class="h-16 flex items-center justify-between px-4 border-b border-gray-800">
                <div class="flex items-center space-x-3">
                    <span class="font-bold text-lg tracking-tight truncate max-w-[120px]">{{ configStore.siteConfig.site_name }}</span>
                    <span class="text-[10px] bg-indigo-600 px-1.5 py-0.5 rounded uppercase font-bold tracking-wider shrink-0">Admin</span>
                </div>
            </div>

            <!-- Navigation -->
            <nav class="flex-1 overflow-y-auto py-4 px-3 space-y-1">
                <router-link
                    v-for="item in navItems"
                    :key="item.path"
                    :to="item.path"
                    class="flex items-center px-3 py-2.5 rounded-lg text-sm font-medium transition-colors"
                    :class="[
                        isActive(item) 
                            ? 'bg-indigo-600 text-white' 
                            : 'text-gray-400 hover:bg-gray-800 hover:text-white'
                    ]"
                >
                    <component :is="item.icon" class="w-5 h-5 mr-3" />
                    {{ item.name }}
                </router-link>
            </nav>

            <!-- Footer -->
            <div class="p-4 border-t border-gray-800">
                <button
                    @click="exitAdmin"
                    class="w-full flex items-center justify-center px-4 py-2.5 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm font-medium text-gray-300 transition-colors"
                >
                    <ArrowLeft class="w-4 h-4 mr-2" />
                    Exit Admin Console
                </button>
                <div class="mt-3 text-xs text-gray-500 text-center">
                    Logged in as <span class="text-gray-300">{{ authStore.user?.username }}</span>
                </div>
            </div>
        </aside>

        <!-- Main Content -->
        <main class="flex-1 overflow-y-auto">
            <div class="max-w-7xl mx-auto px-6 py-8">
                <RouterView />
            </div>
        </main>
    </div>
</template>
