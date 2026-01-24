<script setup lang="ts">
import { computed } from 'vue';
import { useUnreadStore } from '../../stores/unreads';
import { useChannelStore } from '../../stores/channels';
import { useTeamStore } from '../../stores/teams';
import { Hash, ArrowRight, BellOff } from 'lucide-vue-next';

const unreadStore = useUnreadStore();
const channelStore = useChannelStore();
const teamStore = useTeamStore();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const unreadChannels = computed(() => {
    const list = [];
    for (const [channelId, count] of Object.entries(unreadStore.channelUnreads)) {
        if (count > 0) {
            const channel = channelStore.channels.find(c => c.id === channelId);
            if (channel) {
                list.push({
                    id: channelId,
                    name: channel.display_name || channel.name,
                    count: count,
                    mentions: unreadStore.channelMentions[channelId] || 0,
                    teamId: channel.team_id
                });
            }
        }
    }
    return list.sort((a, b) => (b.mentions - a.mentions) || (b.count - a.count));
});

function goToChannel(channel: any) {
    if (teamStore.currentTeamId !== channel.teamId) {
        teamStore.selectTeam(channel.teamId);
    }
    channelStore.selectChannel(channel.id);
    unreadStore.markAsRead(channel.id);
    emit('close');
}

function clearAll() {
    unreadStore.markAllAsRead();
    emit('close');
}
</script>

<template>
    <div class="absolute right-0 mt-2 w-80 bg-white dark:bg-slate-900 border border-gray-200 dark:border-slate-800 rounded-xl shadow-2xl z-50 overflow-hidden">
        <div class="px-4 py-3 border-b border-gray-100 dark:border-slate-800 flex items-center justify-between bg-gray-50/50 dark:bg-slate-800/50">
            <h3 class="text-sm font-bold text-gray-900 dark:text-white">Unread Activity</h3>
            <button 
                v-if="unreadChannels.length > 0"
                @click="clearAll" 
                class="text-[11px] font-medium text-indigo-600 dark:text-indigo-400 hover:underline"
            >
                Mark all as read
            </button>
        </div>

        <div class="max-h-[400px] overflow-y-auto">
            <div v-if="unreadChannels.length === 0" class="py-12 text-center">
                <BellOff class="w-8 h-8 mx-auto text-gray-300 dark:text-slate-700 mb-3" />
                <p class="text-sm text-gray-500 dark:text-slate-400">All caught up!</p>
            </div>

            <div v-else class="divide-y divide-gray-50 dark:divide-slate-800">
                <div 
                    v-for="channel in unreadChannels" 
                    :key="channel.id"
                    @click="goToChannel(channel)"
                    class="px-4 py-3 hover:bg-gray-50 dark:hover:bg-slate-800/50 cursor-pointer transition-colors group"
                >
                    <div class="flex items-center justify-between">
                        <div class="flex items-center min-w-0 mr-2">
                            <Hash class="w-4 h-4 text-gray-400 mr-2 shrink-0" />
                            <span class="text-sm font-medium text-gray-700 dark:text-slate-200 truncate">{{ channel.name }}</span>
                        </div>
                        <div class="flex items-center space-x-2 shrink-0">
                            <span v-if="channel.mentions > 0" class="px-1.5 py-0.5 bg-red-500 text-white text-[10px] font-bold rounded-full">
                                {{ channel.mentions }}
                            </span>
                            <span class="px-1.5 py-0.5 bg-gray-200 dark:bg-slate-700 text-gray-600 dark:text-slate-300 text-[10px] font-bold rounded-full">
                                {{ channel.count }}
                            </span>
                            <ArrowRight class="w-3.5 h-3.5 text-gray-300 dark:text-slate-600 group-hover:text-indigo-500 transition-colors" />
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <div class="px-4 py-2 border-t border-gray-100 dark:border-slate-800 bg-gray-50/30 dark:bg-slate-800/30">
            <p class="text-[10px] text-gray-400 dark:text-slate-500 text-center">
                Showing unread messages from joined channels
            </p>
        </div>
    </div>
</template>
