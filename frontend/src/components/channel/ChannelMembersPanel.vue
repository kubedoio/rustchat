<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { Search, UserPlus, Shield, User } from 'lucide-vue-next';
import RcAvatar from '../ui/RcAvatar.vue';
import api from '../../api/client';
import type { PresenceStatus } from '../../core/entities/User';
import { getPresencePresentation, normalizePresenceStatus } from '../../features/presence/presencePresentation';

const props = defineProps<{
    channelId: string;
}>();

const emit = defineEmits(['close']);

const members = ref<any[]>([]);
const loading = ref(false);
const searchQuery = ref('');

async function fetchMembers() {
    if (!props.channelId) return;
    loading.value = true;
    try {
        const response = await api.get(`/channels/${props.channelId}/members`);
        members.value = response.data;
    } catch (e) {
        console.error('Failed to fetch channel members:', e);
    } finally {
        loading.value = false;
    }
}

const filteredMembers = computed(() => {
    if (!searchQuery.value) return members.value;
    const q = searchQuery.value.toLowerCase();
    return members.value.filter(m => 
        m.username?.toLowerCase().includes(q) || 
        m.display_name?.toLowerCase().includes(q)
    );
});

function getPresenceMeta(presence?: string) {
    const meta = getPresencePresentation(presence);
    return {
        label: meta.label,
        icon: meta.icon,
        badgeClass: meta.status === 'offline'
            ? 'bg-bg-surface-2 text-text-3 border-border-1'
            : `${meta.badgeClass} border`,
    };
}

const presenceSections = computed(() => {
    const order: PresenceStatus[] = ['online', 'away', 'dnd', 'offline'];
    return order
        .map((presence) => ({
            key: presence,
            label: getPresenceMeta(presence).label,
            members: filteredMembers.value.filter(
                (member) => normalizePresenceStatus(member.presence) === presence
            ),
        }))
        .filter((section) => section.members.length > 0);
});

onMounted(fetchMembers);
watch(() => props.channelId, fetchMembers);

</script>

<template>
    <div class="flex h-full flex-col bg-bg-surface-1">
        <!-- Toolbar -->
        <div class="space-y-4 border-b border-border-1 bg-bg-surface-2/60 p-4">
            <div class="relative group">
                <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-3 transition-colors group-focus-within:text-brand" />
                <input 
                    v-model="searchQuery"
                    type="text" 
                    placeholder="Find members" 
                    class="w-full rounded-r-2 border border-border-1 bg-bg-surface-1 py-2 pl-10 pr-3 text-sm text-text-1 shadow-1 transition-standard focus:outline-none focus:ring-2 focus:ring-brand/20 focus:border-brand/50"
                />
            </div>
            <button class="flex w-full items-center justify-center space-x-2 rounded-r-2 bg-brand/10 px-3 py-2 text-sm font-semibold text-brand transition-standard active:scale-[0.98] hover:bg-brand/15">
                <UserPlus class="w-4 h-4" />
                <span>Invite People</span>
            </button>
        </div>

        <!-- Members List -->
        <div class="custom-scrollbar flex-1 space-y-6 overflow-y-auto p-3">
            <div v-for="section in presenceSections" :key="section.key">
                <div class="px-3 pb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-text-3">
                    {{ section.label }} — {{ section.members.length }}
                </div>
                <div class="space-y-1">
                    <div 
                        v-for="member in section.members" 
                        :key="member.user_id"
                        class="group flex cursor-pointer items-center gap-3 rounded-r-2 border border-transparent p-2 transition-standard hover:border-border-1 hover:bg-bg-surface-2/70"
                    >
                        <RcAvatar 
                            :userId="member.user_id"
                            :username="member.username"
                            :src="member.avatar_url"
                            size="sm"
                            class="rounded-lg"
                        />
                        <div class="min-w-0 flex-1">
                            <div class="flex items-center justify-between gap-3">
                                <span class="truncate text-[14px] font-semibold text-text-1 group-hover:text-brand transition-standard">{{ member.display_name || member.username }}</span>
                                <Shield v-if="member.role === 'admin'" class="h-3.5 w-3.5 text-warning" title="Admin" />
                            </div>
                            <div class="mt-1 flex items-center gap-2 text-xs text-text-3">
                                <span class="truncate">@{{ member.username }}</span>
                                <span class="h-1 w-1 rounded-full bg-border-2" />
                                <span class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5" :class="getPresenceMeta(member.presence).badgeClass">
                                    <component :is="getPresenceMeta(member.presence).icon" class="h-3 w-3" />
                                    {{ getPresenceMeta(member.presence).label }}
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Loading Indicator -->
            <div v-if="loading" class="flex flex-col items-center space-y-3 py-10">
                <div class="h-6 w-6 animate-spin rounded-full border-2 border-brand border-t-transparent font-medium uppercase tracking-widest"></div>
                <p class="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-3">Fetching Members</p>
            </div>

            <!-- No results -->
            <div v-if="!loading && filteredMembers.length === 0" class="space-y-4 px-6 py-16 text-center">
                <div class="mx-auto flex h-16 w-16 items-center justify-center rounded-full border border-border-1 bg-bg-surface-2 opacity-60">
                    <User class="h-8 w-8 text-text-3" />
                </div>
                <div>
                   <p class="text-[15px] font-semibold text-text-1">No members found</p>
                   <p class="mt-1 text-xs text-text-3">Try a different search term or check for typos.</p>
                </div>
            </div>
        </div>
    </div>

</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--color-border-1);
  border-radius: 4px;
}
</style>
