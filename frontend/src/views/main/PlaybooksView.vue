<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { Plus, Book, Play, ArrowRight } from 'lucide-vue-next'
import { usePlaybookStore } from '../../stores/playbooks'
import ChannelSidebar from '../../components/layout/ChannelSidebar.vue'
import PlaybookList from '../../components/playbooks/PlaybookList.vue'
import BaseButton from '../../components/atomic/BaseButton.vue'

const router = useRouter()
const playbookStore = usePlaybookStore()
const activeTab = ref<'playbooks' | 'runs'>('playbooks')

onMounted(() => {
    playbookStore.fetchPlaybooks()
    playbookStore.fetchRuns()
})
</script>

<template>
    <div class="flex h-screen bg-white dark:bg-gray-900">
        <!-- Sidebar (Reusing ChannelSidebar for navigation consistency) -->
        <ChannelSidebar />

        <!-- Main Content -->
        <div class="flex-1 flex flex-col min-w-0 bg-gray-50 dark:bg-gray-800">
            <!-- Header -->
            <header class="bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex justify-between items-center">
                <div class="flex items-center space-x-4">
                    <div class="p-2 bg-indigo-100 dark:bg-indigo-900/30 rounded-lg">
                        <Book class="w-6 h-6 text-indigo-600 dark:text-indigo-400" />
                    </div>
                    <div>
                        <h1 class="text-xl font-bold text-gray-900 dark:text-white">Playbooks</h1>
                        <p class="text-sm text-gray-500 dark:text-gray-400">Automate your team's workflows</p>
                    </div>
                </div>
                
                <BaseButton @click="router.push('/playbooks/new')">
                    <Plus class="w-4 h-4 mr-2" />
                    Create Playbook
                </BaseButton>
            </header>

            <!-- Tabs -->
            <div class="px-6 mt-4 border-b border-gray-200 dark:border-gray-700">
                <nav class="flex space-x-8">
                    <button
                        @click="activeTab = 'playbooks'"
                        class="pb-4 border-b-2 font-medium text-sm transition-colors flex items-center"
                        :class="activeTab === 'playbooks' 
                            ? 'border-indigo-500 text-indigo-600 dark:text-indigo-400' 
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'"
                    >
                        <Book class="w-4 h-4 mr-2" />
                        Library
                    </button>
                    <button
                        @click="activeTab = 'runs'"
                        class="pb-4 border-b-2 font-medium text-sm transition-colors flex items-center"
                        :class="activeTab === 'runs' 
                            ? 'border-indigo-500 text-indigo-600 dark:text-indigo-400' 
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'"
                    >
                        <Play class="w-4 h-4 mr-2" />
                        Runs
                    </button>
                </nav>
            </div>

            <!-- Content Area -->
            <div class="flex-1 overflow-y-auto p-6">
                <Transition
                    enter-active-class="transition ease-out duration-100"
                    enter-from-class="opacity-0 translate-y-1"
                    enter-to-class="opacity-100 translate-y-0"
                    leave-active-class="transition ease-in duration-75"
                    leave-from-class="opacity-100 translate-y-0"
                    leave-to-class="opacity-0 translate-y-1"
                    mode="out-in"
                >
                    <div v-if="activeTab === 'playbooks'" key="playbooks">
                        <PlaybookList />
                    </div>
                    <div v-else key="runs">
                        <div v-if="playbookStore.runs.length === 0" class="text-center py-20 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 border-dashed">
                            <Play class="w-12 h-12 text-gray-300 mx-auto mb-3" />
                            <h3 class="text-lg font-medium text-gray-900 dark:text-white">No active runs</h3>
                            <p class="text-gray-500">Start a playbook run to track progress</p>
                        </div>
                        <div v-else class="grid grid-cols-1 gap-4">
                            <div 
                                v-for="run in playbookStore.runs" 
                                :key="run.id"
                                @click="router.push(`/runs/${run.id}`)"
                                class="bg-white dark:bg-gray-800 p-4 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm hover:shadow-md transition-all cursor-pointer flex items-center justify-between group"
                            >
                                <div class="flex items-center space-x-4">
                                    <div class="p-2 bg-blue-50 dark:bg-blue-900/30 rounded-lg text-blue-600 dark:text-blue-400">
                                        <Play class="w-5 h-5" />
                                    </div>
                                    <div>
                                        <h3 class="font-medium text-gray-900 dark:text-white group-hover:text-primary transition-colors">{{ run.name }}</h3>
                                        <div class="flex items-center space-x-2 mt-1 text-xs text-gray-500">
                                            <span class="capitalize px-1.5 py-0.5 rounded bg-gray-100 dark:bg-gray-700">{{ run.status.replace('_', ' ') }}</span>
                                            <span>• Started {{ new Date(run.started_at).toLocaleDateString() }}</span>
                                        </div>
                                    </div>
                                </div>
                                <div class="text-gray-400">
                                    <ArrowRight class="w-5 h-5 group-hover:translate-x-1 transition-transform" />
                                </div>
                            </div>
                        </div>
                    </div>
                </Transition>
            </div>
        </div>
    </div>
</template>
