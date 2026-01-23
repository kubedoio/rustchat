<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { Plus, Clock, Play } from 'lucide-vue-next'
import { usePlaybookStore } from '../../stores/playbooks'
import BaseButton from '../atomic/BaseButton.vue'

const router = useRouter()
const playbookStore = usePlaybookStore()

const playbooks = computed(() => playbookStore.playbooks)

function formatDate(date: string) {
    return new Date(date).toLocaleDateString()
}

async function handleStartRun(playbook: any) {
    const runName = prompt(`Name for this run of ${playbook.name}:`, `${playbook.name} Run`)
    if (runName) {
        try {
            const result = await playbookStore.startRun(playbook.id, runName)
            if (result) {
                router.push(`/runs/${result.run.id}`)
            }
        } catch (e) {
            console.error('Failed to start run', e)
        }
    }
}
</script>

<template>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <!-- New Playbook Card -->
        <button 
            @click="router.push('/playbooks/new')"
            class="flex flex-col items-center justify-center p-8 border-2 border-dashed border-gray-300 dark:border-gray-700 rounded-xl hover:border-indigo-500 hover:bg-gray-50 dark:hover:bg-gray-800 transition-all group"
        >
            <div class="w-12 h-12 rounded-full bg-indigo-50 dark:bg-indigo-900/30 flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
                <Plus class="w-6 h-6 text-indigo-600 dark:text-indigo-400" />
            </div>
            <h3 class="font-semibold text-gray-900 dark:text-white">Create New Playbook</h3>
            <p class="text-sm text-gray-500 mt-1">Start from scratch or template</p>
        </button>

        <!-- Playbook Cards -->
        <div 
            v-for="playbook in playbooks" 
            :key="playbook.id"
            class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm hover:shadow-md transition-shadow p-6 flex flex-col"
        >
            <div class="flex items-start justify-between mb-4">
                <div class="flex items-center space-x-3">
                    <span class="text-3xl">{{ playbook.icon || '📘' }}</span>
                    <div>
                        <h3 class="font-semibold text-gray-900 dark:text-white line-clamp-1">{{ playbook.name }}</h3>
                        <p class="text-xs text-gray-500 flex items-center mt-1">
                            <Clock class="w-3 h-3 mr-1" />
                            Updated {{ formatDate(playbook.updated_at) }}
                        </p>
                    </div>
                </div>
            </div>
            
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-6 line-clamp-2 min-h-[40px]">
                {{ playbook.description || 'No description provided' }}
            </p>

            <div class="mt-auto flex space-x-2">
                <BaseButton 
                    class="w-full justify-center" 
                    size="sm"
                    @click="handleStartRun(playbook)"
                >
                    <Play class="w-4 h-4 mr-1" />
                    Run
                </BaseButton>
                <BaseButton 
                    variant="secondary" 
                    class="w-full justify-center" 
                    size="sm"
                    @click="router.push(`/playbooks/${playbook.id}/edit`)"
                >
                    Edit
                </BaseButton>
            </div>
        </div>
    </div>
</template>
