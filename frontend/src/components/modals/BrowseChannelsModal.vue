<script setup lang="ts">
import { ref, computed, watch } from 'vue';
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
const activeTeamId = computed(() => teamStore.currentTeamId || teamStore.currentTeam?.id || null);

watch([() => props.open, activeTeamId], ([isOpen, teamId]) => {
    if (isOpen && teamId) {
        channelStore.fetchJoinableChannels(teamId);
    }
}, { immediate: true });

const availableChannels = computed(() => channelStore.joinableChannels);

async function joinChannel(channelId: string) {
    joining.value = channelId;
    try {
        await channelStore.joinChannel(channelId);
        if (activeTeamId.value) {
            // Refresh my channels
            await channelStore.fetchChannels(activeTeamId.value);
            // Refresh joinable list
            await channelStore.fetchJoinableChannels(activeTeamId.value);
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
            <div class="absolute inset-0 bg-bg-app/70 backdrop-blur-sm" @click="emit('close')"></div>
            
            <!-- Modal -->
            <div class="relative mx-4 max-h-[80vh] w-full max-w-lg overflow-hidden rounded-r-3 border border-border-1 bg-bg-surface-1 shadow-2xl">
                <!-- Header -->
                <div class="flex items-center justify-between border-b border-border-1 px-6 py-4">
                    <div class="flex items-center space-x-3">
                        <div class="flex h-10 w-10 items-center justify-center rounded-r-2 bg-brand/10 text-brand">
                            <Hash class="h-5 w-5" />
                        </div>
                        <div>
                            <h2 class="text-lg font-semibold text-brand">Browse Channels</h2>
                            <p class="text-xs text-text-3">Discover spaces in this workspace and join the right conversation.</p>
                        </div>
                    </div>
                    <button @click="emit('close')" class="rounded-r-2 p-1 transition-standard hover:bg-bg-surface-2">
                        <X class="h-5 w-5 text-text-3" />
                    </button>
                </div>

                <!-- Content -->
                <div class="max-h-[60vh] overflow-y-auto p-6">
                    <div v-if="channelStore.loading" class="py-8 text-center text-text-3">
                        Loading channels...
                    </div>

                    <div v-else-if="availableChannels.length === 0" class="py-8 text-center text-text-3">
                        <Hash class="mx-auto mb-4 h-12 w-12 text-text-4" />
                        <p>No new channels to join</p>
                    </div>

                    <div v-else class="space-y-4">
                        <div 
                            v-for="channel in availableChannels" 
                            :key="channel.id"
                            class="flex items-center justify-between rounded-r-2 border border-border-1 bg-bg-surface-2/70 p-4 transition-standard hover:bg-bg-surface-2"
                        >
                            <div class="flex-1 min-w-0">
                                <p class="flex items-center truncate font-medium text-text-1">
                                    <Hash class="mr-1 h-4 w-4 text-text-3" />
                                    {{ channel.display_name || channel.name }}
                                </p>
                                <p v-if="channel.purpose" class="mt-0.5 ml-5 truncate text-sm text-text-3">
                                    {{ channel.purpose }}
                                </p>
                            </div>
                            <button
                                @click="joinChannel(channel.id)"
                                :disabled="joining === channel.id"
                                class="ml-4 flex items-center rounded-r-2 bg-brand px-3 py-1.5 text-sm font-medium text-brand-foreground transition-standard hover:opacity-90 disabled:opacity-50"
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
