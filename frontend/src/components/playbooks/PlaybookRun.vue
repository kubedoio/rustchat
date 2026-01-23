<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, CheckCircle2, Circle, Send } from 'lucide-vue-next'
import { useToast } from '../../composables/useToast'
import { playbooksApi, type RunWithTasks, type RunStatusUpdate, type RunTask } from '../../api/playbooks'

const route = useRoute()
const router = useRouter()
const toast = useToast()

const run = ref<RunWithTasks | null>(null)
const updates = ref<RunStatusUpdate[]>([])
const newUpdate = ref('')
const loading = ref(true)
const sendingUpdate = ref(false)

const progressPercent = computed(() => {
    if (!run.value) return 0
    return Math.round((run.value.progress.completed / run.value.progress.total) * 100) || 0
})

const taskMap = ref<Record<string, { title: string, description: string | null }>>({})

onMounted(async () => {
    const runId = route.params.id as string
    if (runId) {
        try {
            const response = await playbooksApi.getRun(runId)
            run.value = response.data
            
            // Fetch playbook to get task details
            if (run.value) {
                const playbookRes = await playbooksApi.get(run.value.run.playbook_id)
                // Flatten tasks
                playbookRes.data.checklists.forEach(c => {
                    c.tasks.forEach(t => {
                        taskMap.value[t.id] = { title: t.title, description: t.description }
                    })
                })
            }

            await fetchUpdates()
        } catch (e) {
            toast.error('Error', 'Failed to load run')
            router.push('/playbooks')
        } finally {
            loading.value = false
        }
    }
})

async function fetchUpdates() {
    if (!run.value) return
    try {
        const response = await playbooksApi.listStatusUpdates(run.value.run.id)
        updates.value = response.data
    } catch(e) {
        console.error('Failed to fetch updates', e)
    }
}

async function postUpdate() {
    if (!run.value || !newUpdate.value.trim()) return
    sendingUpdate.value = true
    try {
        const response = await playbooksApi.createStatusUpdate(run.value.run.id, {
            message: newUpdate.value,
            is_broadcast: false
        })
        updates.value.unshift(response.data)
        newUpdate.value = ''
    } catch (e) {
        toast.error('Error', 'Failed to post update')
    } finally {
        sendingUpdate.value = false
    }
}

async function toggleTask(task: RunTask) {
    if (!run.value) return
    const newStatus = task.status === 'done' ? 'pending' : 'done'
    
    // Optimistic update
    const oldStatus = task.status
    task.status = newStatus
    
    // Update progress locally
    if (newStatus === 'done') {
        run.value.progress.completed++
        run.value.progress.pending--
    } else {
        run.value.progress.completed--
        run.value.progress.pending++
    }

    try {
        await playbooksApi.updateRunTask(run.value.run.id, task.task_id, {
            status: newStatus
        })
    } catch (e) {
        // Revert
        task.status = oldStatus
        toast.error('Error', 'Failed to update task')
    }
}

function formatDate(date: string) {
    return new Date(date).toLocaleString()
}
</script>

<template>
    <div class="h-full flex flex-col bg-gray-50 dark:bg-gray-900">
        <div v-if="loading" class="flex-1 flex items-center justify-center">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
        </div>

        <template v-else-if="run">
            <!-- Header -->
            <header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
                <div class="flex items-center space-x-4 mb-4">
                    <button @click="router.push('/playbooks')" class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg">
                        <ArrowLeft class="w-5 h-5 text-gray-500" />
                    </button>
                    <div>
                        <div class="flex items-center space-x-2">
                            <h1 class="text-xl font-bold text-gray-900 dark:text-white">{{ run.run.name }}</h1>
                            <span class="px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300 capitalize">
                                {{ run.run.status.replace('_', ' ') }}
                            </span>
                        </div>
                        <p class="text-sm text-gray-500">Started {{ formatDate(run.run.started_at) }}</p>
                    </div>
                </div>

                <!-- Progress Bar -->
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5 mb-1">
                    <div class="bg-primary h-2.5 rounded-full transition-all duration-500" :style="{ width: `${progressPercent}%` }"></div>
                </div>
                <div class="flex justify-between text-xs text-gray-500">
                    <span>{{ progressPercent }}% Complete</span>
                    <span>{{ run.progress.completed }}/{{ run.progress.total }} tasks</span>
                </div>
            </header>

            <div class="flex-1 flex overflow-hidden">
                <!-- Tasks List -->
                <div class="flex-1 overflow-y-auto p-6 border-r border-gray-200 dark:border-gray-700">
                    <h2 class="text-lg font-semibold mb-4 text-gray-900 dark:text-white">Tasks</h2>
                    <div class="space-y-4">
                        <div 
                            v-for="task in run.tasks" 
                            :key="task.id"
                            class="bg-white dark:bg-gray-800 p-4 rounded-lg border border-gray-200 dark:border-gray-700 flex items-start space-x-3 hover:shadow-sm transition-shadow"
                            :class="{ 'opacity-75': task.status === 'done' }"
                        >
                            <button 
                                @click="toggleTask(task)"
                                class="mt-1 flex-shrink-0 text-gray-400 hover:text-primary transition-colors"
                                :class="{ 'text-green-500': task.status === 'done' }"
                            >
                                <CheckCircle2 v-if="task.status === 'done'" class="w-5 h-5" />
                                <Circle v-else class="w-5 h-5" />
                            </button>
                            
                            <div class="flex-1">
                                <p class="text-gray-900 dark:text-white font-medium" :class="{ 'line-through text-gray-500': task.status === 'done' }">
                                    {{ taskMap[task.task_id]?.title || 'Unknown Task' }}
                                </p>
                                <p v-if="taskMap[task.task_id]?.description" class="text-sm text-gray-500 mt-0.5">{{ taskMap[task.task_id]?.description }}</p>
                                <p v-if="task.notes" class="text-sm text-gray-500 mt-1 italic">{{ task.notes }}</p>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Sidebar: Status Updates -->
                <div class="w-80 bg-white dark:bg-gray-800 flex flex-col border-l border-gray-200 dark:border-gray-700">
                    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Timeline</h2>
                    </div>

                    <div class="flex-1 overflow-y-auto p-4 space-y-4">
                        <div v-for="update in updates" :key="update.id" class="flex space-x-3">
                            <div class="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center flex-shrink-0">
                                <span class="text-xs font-medium text-gray-600 dark:text-gray-300">U</span>
                            </div>
                            <div>
                                <div class="bg-gray-100 dark:bg-gray-700/50 rounded-lg p-3 text-sm text-gray-800 dark:text-gray-200">
                                    {{ update.message }}
                                </div>
                                <p class="text-xs text-gray-500 mt-1">{{ formatDate(update.created_at) }}</p>
                            </div>
                        </div>
                    </div>

                    <div class="p-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
                        <form @submit.prevent="postUpdate" class="flex space-x-2">
                            <input 
                                v-model="newUpdate"
                                placeholder="Post an update..." 
                                class="flex-1 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
                            />
                            <button 
                                type="submit" 
                                :disabled="!newUpdate || sendingUpdate"
                                class="p-2 bg-primary text-white rounded-lg hover:bg-primary/90 disabled:opacity-50"
                            >
                                <Send class="w-4 h-4" />
                            </button>
                        </form>
                    </div>
                </div>
            </div>
        </template>
    </div>
</template>
