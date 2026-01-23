<script setup lang="ts">
import { X } from 'lucide-vue-next';
import { useUIStore } from '../../stores/ui';
import ThreadPanel from '../channel/ThreadPanel.vue';

const ui = useUIStore();
</script>

<template>
  <aside class="w-[400px] bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-800 flex flex-col shadow-xl z-20 shrink-0">
    <!-- Header -->
    <div class="h-14 border-b border-gray-200 dark:border-gray-800 flex items-center justify-between px-4 shrink-0 bg-gray-50 dark:bg-gray-900/50">
        <h3 class="font-bold text-gray-700 dark:text-gray-200">
            <span v-if="ui.rhsView === 'thread'">Thread</span>
            <span v-else-if="ui.rhsView === 'search'">Search Results</span>
            <span v-else-if="ui.rhsView === 'info'">Channel Info</span>
        </h3>
        <button @click="ui.closeRhs()" class="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded text-gray-500">
            <X class="w-5 h-5" />
        </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto flex flex-col bg-gray-50/50 dark:bg-gray-900/20">
         <ThreadPanel v-if="ui.rhsView === 'thread'" />

         <div v-else-if="ui.rhsView === 'search'" class="flex-1 flex flex-col p-4 text-center text-gray-500">
             <p>Search results will appear here.</p>
         </div>

         <div v-else-if="ui.rhsView === 'info'" class="flex-1 flex flex-col p-4 text-center text-gray-500">
             <p>Channel details and members.</p>
         </div>
         
         <div v-else class="flex-1 flex items-center justify-center text-gray-400">
             <p class="text-sm">No content</p>
         </div>
    </div>
  </aside>
</template>
