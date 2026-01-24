<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { X, Hash, ArrowRight } from 'lucide-vue-next';
import { useChannelStore } from '../../stores/channels';
import { useTeamStore } from '../../stores/teams';
import { useToast } from '../../composables/useToast';

const props = defineProps<{
    open: boolean
}>();

const emit = defineEmits<{
    (e: 'close'): void
}>();

const channelStore = useChannelStore();
const teamStore = useTeamStore();
const toast = useToast();
const joining = ref<string | null>(null);

onMounted(() => {
    if (teamStore.currentTeam?.id) {
        channelStore.fetchJoinableChannels(teamStore.currentTeam.id);
    }
});

const availableChannels = computed(() => channelStore.joinableChannels);

async function joinChannel(channelId: string) {
    joining.value = channelId;
    try {
        await channelStore.joinChannel(channelId);
        if (teamStore.currentTeam?.id) {
            // Refresh my channels
            await channelStore.fetchChannels(teamStore.currentTeam.id);
            // Refresh joinable list
            await channelStore.fetchJoinableChannels(teamStore.currentTeam.id);
        }
        emit('close');
        toast.success('Joined channel', 'You have successfully joined the channel');
    } catch (e) {
        console.error('Failed to join channel:', e);
        toast.error('Failed to join', 'Could not join the channel');
    } finally {
        joining.value = null;
    }
}
</script>

<template>
    <Teleport to="body">
        <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center">
            <!-- Backdrop -->
            <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="emit('close')"></div>
            
            <!-- Modal -->
            <div class="relative bg-slate-900 rounded-xl shadow-2xl border border-white/10 w-full max-w-lg max-h-[80vh] overflow-hidden">
                <!-- Header -->
                <div class="flex items-center justify-between px-6 py-4 border-b border-white/10">
                    <div class="flex items-center space-x-3">
                        <Hash class="w-5 h-5 text-indigo-400" />
                        <h2 class="text-lg font-semibold text-white">Browse Channels</h2>
                    </div>
                    <button @click="emit('close')" class="p-1 hover:bg-white/10 rounded-lg transition-colors">
                        <X class="w-5 h-5 text-gray-400" />
                    </button>
                </div>

                <!-- Content -->
                <div class="p-6 overflow-y-auto max-h-[60vh]">
                    <div v-if="channelStore.loading" class="text-center py-8 text-gray-400">
                        Loading channels...
                    </div>

                    <div v-else-if="availableChannels.length === 0" class="text-center py-8 text-gray-400">
                        <Hash class="w-12 h-12 mx-auto mb-4 text-gray-600" />
                        <p>No new channels to join</p>
                    </div>

                    <div v-else class="space-y-4">
                        <div 
                            v-for="channel in availableChannels" 
                            :key="channel.id"
                            class="flex items-center justify-between p-4 bg-slate-800 hover:bg-slate-700 rounded-lg transition-colors"
                        >
                            <div class="flex-1 min-w-0">
                                <p class="font-medium text-white truncate flex items-center">
                                    <Hash class="w-4 h-4 mr-1 text-gray-500" />
                                    {{ channel.display_name || channel.name }}
                                </p>
                                <p v-if="channel.purpose" class="text-sm text-gray-400 truncate mt-0.5 ml-5">
                                    {{ channel.purpose }}
                                </p>
                            </div>
                            <button
                                @click="joinChannel(channel.id)"
                                :disabled="joining === channel.id"
                                class="ml-4 flex items-center px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
                            >
                                <span>{{ joining === channel.id ? 'Joining...' : 'Join' }}</span>
                                <ArrowRight class="w-4 h-4 ml-1" />
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </Teleport>
</template>
