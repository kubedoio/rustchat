<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { Search, X, UserPlus, Shield, User } from 'lucide-vue-next';
import RcAvatar from '../ui/RcAvatar.vue';
import api from '../../api/client';

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

const onlineMembers = computed(() => filteredMembers.value.filter(m => m.presence === 'online' || m.presence === 'dnd'));
const offlineMembers = computed(() => filteredMembers.value.filter(m => m.presence !== 'online' && m.presence !== 'dnd'));

onMounted(fetchMembers);
watch(() => props.channelId, fetchMembers);

</script>

<template>
    <div class="flex flex-col h-full bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-800 w-80">
        <!-- Header -->
        <div class="h-[60px] flex items-center justify-between px-4 border-b border-gray-200 dark:border-gray-800 shrink-0">
            <h3 class="font-bold text-gray-900 dark:text-white flex items-center">
                Members
                <span class="ml-2 px-1.5 py-0.5 bg-gray-100 dark:bg-gray-800 text-gray-500 rounded text-[10px]">{{ members.length }}</span>
            </h3>
            <button @click="$emit('close')" class="p-1 hover:bg-gray-100 dark:hover:bg-gray-800 rounded text-gray-500">
                <X class="w-5 h-5" />
            </button>
        </div>

        <!-- Toolbar -->
        <div class="p-3 border-b border-gray-100 dark:border-gray-800 space-y-3">
            <div class="relative">
                <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
                <input 
                    v-model="searchQuery"
                    type="text" 
                    placeholder="Find members" 
                    class="w-full bg-gray-50 dark:bg-gray-800 border-none rounded-md pl-9 pr-3 py-1.5 text-sm focus:ring-1 focus:ring-primary dark:text-gray-200"
                />
            </div>
            <button class="w-full flex items-center justify-center space-x-2 py-1.5 px-3 bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 rounded-md text-sm font-medium hover:bg-indigo-100 dark:hover:bg-indigo-900/50 transition-colors">
                <UserPlus class="w-4 h-4" />
                <span>Add People</span>
            </button>
        </div>

        <!-- Members List -->
        <div class="flex-1 overflow-y-auto custom-scrollbar p-2 space-y-4">
            <!-- Online Members -->
            <div v-if="onlineMembers.length > 0">
                <div class="px-2 pb-1 text-[10px] font-bold text-gray-500 uppercase tracking-wider">Online</div>
                <div class="space-y-0.5">
                    <div 
                        v-for="member in onlineMembers" 
                        :key="member.user_id"
                        class="flex items-center space-x-3 p-2 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer group"
                    >
                        <RcAvatar 
                            :userId="member.user_id"
                            :username="member.username"
                            :src="member.avatar_url"
                            size="sm"
                        />
                        <div class="flex-1 min-w-0">
                            <div class="flex items-center justify-between">
                                <span class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{{ member.display_name || member.username }}</span>
                                <Shield v-if="member.role === 'admin'" class="w-3 h-3 text-amber-500" />
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Offline Members -->
            <div v-if="offlineMembers.length > 0">
                <div class="px-2 pb-1 text-[10px] font-bold text-gray-500 uppercase tracking-wider">Offline</div>
                <div class="space-y-0.5">
                    <div 
                        v-for="member in offlineMembers" 
                        :key="member.user_id"
                        class="flex items-center space-x-3 p-2 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer group opacity-60"
                    >
                        <RcAvatar 
                            :userId="member.user_id"
                            :username="member.username"
                            :src="member.avatar_url"
                            size="sm"
                            :showPresence="false"
                        />
                        <div class="flex-1 min-w-0">
                            <div class="flex items-center justify-between">
                                <span class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{{ member.display_name || member.username }}</span>
                                <Shield v-if="member.role === 'admin'" class="w-3 h-3 text-amber-500" />
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Loading Indicator -->
            <div v-if="loading" class="py-4 flex justify-center">
                <div class="animate-spin w-5 h-5 border-2 border-primary border-t-transparent rounded-full"></div>
            </div>

            <!-- No results -->
            <div v-if="!loading && filteredMembers.length === 0" class="py-10 text-center space-y-2">
                <User class="w-8 h-8 text-gray-300 mx-auto" />
                <p class="text-xs text-gray-500">No members found</p>
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
  background: #E2E8F0;
  border-radius: 4px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: #334155;
}
</style>
