<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { Plus, Trash2, GripVertical, CheckSquare, Save, ArrowLeft, X } from 'lucide-vue-next'
import { usePlaybookStore } from '../../stores/playbooks'
import BaseButton from '../atomic/BaseButton.vue'
import { useToast } from '../../composables/useToast'
import { type ChecklistWithTasks, playbooksApi } from '../../api/playbooks'

const router = useRouter()
const route = useRoute()
const playbookStore = usePlaybookStore()
const toast = useToast()

const props = defineProps<{
    id?: string // If present, editing existing playbook
}>()

const loading = ref(false)
const saving = ref(false)

const form = ref({
    name: '',
    description: '',
    icon: '📘',
    is_public: false,
    create_channel_on_run: true,
    channel_name_template: 'incident-{{date}}',
    keyword_triggers: '',
})

const checklists = ref<ChecklistWithTasks[]>([])

const isEditing = computed(() => !!props.id || !!route.params.id)

onMounted(async () => {
    const playbookId = (props.id || route.params.id) as string
    if (playbookId) {
        loading.value = true
        try {
            await playbookStore.fetchPlaybook(playbookId)
            const playbook = playbookStore.currentPlaybook
            if (playbook) {
                form.value = {
                    name: playbook.name,
                    description: playbook.description || '',
                    icon: playbook.icon || '📘',
                    is_public: playbook.is_public,
                    create_channel_on_run: playbook.create_channel_on_run,
                    channel_name_template: playbook.channel_name_template || '',
                    keyword_triggers: playbook.keyword_triggers?.join(', ') || '',
                }
                checklists.value = JSON.parse(JSON.stringify(playbook.checklists))
            }
        } catch (e) {
            toast.error('Error', 'Failed to load playbook')
        } finally {
            loading.value = false
        }
    } else {
        // Add default checklist for new playbook
        checklists.value = [{
            id: 'temp-1',
            playbook_id: '',
            name: 'Default Checklist',
            sort_order: 1,
            created_at: new Date().toISOString(),
            tasks: []
        }]
    }
})

function addChecklist() {
    checklists.value.push({
        id: `temp-${Date.now()}`,
        playbook_id: '',
        name: 'New Checklist',
        sort_order: checklists.value.length + 1,
        created_at: new Date().toISOString(),
        tasks: []
    })
}

function removeChecklist(index: number) {
    if (checklists.value[index]) {
        checklists.value.splice(index, 1)
    }
}

function addTask(checklistIndex: number) {
    const checklist = checklists.value[checklistIndex]
    if (checklist) {
        checklist.tasks.push({
            id: `temp-task-${Date.now()}`,
            checklist_id: checklist.id,
            title: '',
            description: '',
            default_assignee_id: null,
            assignee_role: null,
            due_after_minutes: null,
            slash_command: null,
            webhook_url: null,
            sort_order: checklist.tasks.length + 1,
            created_at: new Date().toISOString()
        })
    }
}

function removeTask(checklistIndex: number, taskIndex: number) {
    const checklist = checklists.value[checklistIndex]
    if (checklist && checklist.tasks[taskIndex]) {
        checklist.tasks.splice(taskIndex, 1)
    }
}

async function save() {
    if (!form.value.name) {
        toast.error('Validation error', 'Playbook name is required')
        return
    }

    saving.value = true
    try {
        let playbookId = props.id || route.params.id as string
        
        const payload = {
            ...form.value,
            keyword_triggers: form.value.keyword_triggers.split(',').map(s => s.trim()).filter(s => s.length > 0)
        }
        
        // 1. Create or Update Playbook
        if (isEditing.value) {
            await playbookStore.updatePlaybook(playbookId, payload)
        } else {
            const newPlaybook = await playbookStore.createPlaybook(payload)
            if (newPlaybook) {
                playbookId = newPlaybook.id
            }
        }

        // 2. Sync Checklists and Tasks
        // We iterate through checklists and create them if they have temp IDs
        // For tasks, we do the same
        if (playbookId) {
            // Simplified sync logic for MVP - just create new checklists/tasks that are temp
            // Real implementation would need proper diffing/updating
            for (const checklist of checklists.value) {
                if (checklist.id.startsWith('temp-')) {
                    const newChecklist = await playbooksApi.createChecklist(playbookId, {
                        name: checklist.name,
                        sort_order: checklist.sort_order
                    })
                    // Create tasks for this new checklist
                    for (const task of checklist.tasks) {
                        await playbooksApi.createTask(newChecklist.data.id, {
                            title: task.title,
                            description: task.description,
                            sort_order: task.sort_order
                        })
                    }
                } else {
                    // Existing checklist, check for new tasks
                    for (const task of checklist.tasks) {
                        if (task.id.startsWith('temp-')) {
                            await playbooksApi.createTask(checklist.id, {
                                title: task.title,
                                description: task.description,
                                sort_order: task.sort_order
                            })
                        }
                    }
                }
            }
        }
        
        toast.success('Saved', 'Playbook saved successfully')
        router.push('/playbooks')
    } catch (e: any) {
        toast.error('Error', e.message || 'Failed to save playbook')
    } finally {
        saving.value = false
    }
}
</script>

<template>
    <div class="h-full flex flex-col bg-gray-50 dark:bg-gray-900">
        <!-- Header -->
        <header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex justify-between items-center">
            <div class="flex items-center space-x-4">
                <button @click="router.back()" class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg">
                    <ArrowLeft class="w-5 h-5 text-gray-500" />
                </button>
                <div>
                    <h1 class="text-xl font-bold text-gray-900 dark:text-white">
                        {{ isEditing ? 'Edit Playbook' : 'Create Playbook' }}
                    </h1>
                </div>
            </div>
            <BaseButton @click="save" :loading="saving">
                <Save class="w-4 h-4 mr-2" />
                Save Playbook
            </BaseButton>
        </header>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-6 max-w-4xl mx-auto w-full">
            <div class="space-y-8">
                <!-- General Settings -->
                <section class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 shadow-sm">
                    <h2 class="text-lg font-semibold mb-4 text-gray-900 dark:text-white">General Information</h2>
                    <div class="space-y-4">
                        <div class="flex space-x-4">
                            <div class="w-20">
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Icon</label>
                                <input v-model="form.icon" class="w-full text-center text-2xl px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                            </div>
                            <div class="flex-1">
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Name</label>
                                <input v-model="form.name" type="text" placeholder="e.g. Incident Response" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                            </div>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description</label>
                            <textarea v-model="form.description" rows="2" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"></textarea>
                        </div>
                    </div>
                </section>

                <!-- Automation Settings -->
                <section class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 shadow-sm">
                    <h2 class="text-lg font-semibold mb-4 text-gray-900 dark:text-white">Automation & Settings</h2>
                    <div class="space-y-4">
                        <div class="flex items-center space-x-2">
                            <input v-model="form.is_public" type="checkbox" id="is_public" class="rounded border-gray-300 text-primary focus:ring-primary" />
                            <label for="is_public" class="text-sm font-medium text-gray-700 dark:text-gray-300">Public (visible to everyone in team)</label>
                        </div>

                        <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
                            <div class="flex items-center space-x-2 mb-2">
                                <input v-model="form.create_channel_on_run" type="checkbox" id="create_channel" class="rounded border-gray-300 text-primary focus:ring-primary" />
                                <label for="create_channel" class="text-sm font-medium text-gray-700 dark:text-gray-300">Create new channel on run</label>
                            </div>
                            
                            <div v-if="form.create_channel_on_run" class="ml-6">
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Channel Name Template</label>
                                <input v-model="form.channel_name_template" type="text" placeholder="incident-{{date}}" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                                <p class="text-xs text-gray-500 mt-1" v-pre>Available variables: {{date}}, {{playbook_name}}</p>
                            </div>
                        </div>

                        <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Keyword Triggers</label>
                            <input v-model="form.keyword_triggers" type="text" placeholder="e.g. incident, outage, server down" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                            <p class="text-xs text-gray-500 mt-1">Comma separated keywords that will trigger a prompt to start this playbook.</p>
                        </div>
                    </div>
                </section>

                <!-- Checklists & Tasks -->
                <section class="space-y-6">
                    <div class="flex items-center justify-between">
                        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">Checklists & Tasks</h2>
                        <BaseButton variant="secondary" size="sm" @click="addChecklist">
                            <Plus class="w-4 h-4 mr-2" />
                            Add Checklist
                        </BaseButton>
                    </div>

                    <div v-for="(checklist, cIndex) in checklists" :key="checklist.id" 
                         class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden shadow-sm">
                        <!-- Checklist Header -->
                        <div class="bg-gray-50 dark:bg-gray-700/50 p-4 flex items-center space-x-3 border-b border-gray-200 dark:border-gray-700">
                            <GripVertical class="w-5 h-5 text-gray-400 cursor-move" />
                            <input v-model="checklist.name" class="flex-1 bg-transparent font-medium text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary rounded px-2" placeholder="Checklist Name" />
                            <button @click="removeChecklist(cIndex)" class="text-gray-400 hover:text-red-500">
                                <Trash2 class="w-4 h-4" />
                            </button>
                        </div>

                        <!-- Tasks -->
                        <div class="p-4 space-y-2">
                            <div v-for="(task, tIndex) in checklist.tasks" :key="task.id" class="flex items-start space-x-3 group">
                                <CheckSquare class="w-5 h-5 text-gray-400 mt-2" />
                                <div class="flex-1 space-y-1">
                                    <input v-model="task.title" class="w-full bg-transparent border-b border-transparent hover:border-gray-200 dark:hover:border-gray-600 focus:border-primary focus:outline-none py-1 text-gray-900 dark:text-white" placeholder="Task title..." />
                                    <input v-model="task.description" class="w-full text-sm bg-transparent border-none focus:ring-0 p-0 text-gray-500" placeholder="Description (optional)" />
                                </div>
                                <button @click="removeTask(cIndex, tIndex)" class="opacity-0 group-hover:opacity-100 text-gray-400 hover:text-red-500 p-1">
                                    <X class="w-4 h-4" />
                                </button>
                            </div>
                            
                            <button @click="addTask(cIndex)" class="flex items-center text-sm text-gray-500 hover:text-primary mt-2 px-1">
                                <Plus class="w-4 h-4 mr-1" />
                                Add Task
                            </button>
                        </div>
                    </div>
                </section>
            </div>
        </div>
    </div>
</template>
