<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '../stores/app'

const store = useAppStore()

onMounted(() => store.fetchHistory())

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleString()
}
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-2xl font-bold text-gray-800">Execution History</h2>
      <button
        @click="store.fetchHistory()"
        class="px-3 py-1.5 text-sm bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 transition-colors cursor-pointer"
      >
        🔄 Refresh
      </button>
    </div>

    <div v-if="store.history.length === 0" class="text-gray-400 text-center py-12">
      No executions yet. Tasks will appear here after they run.
    </div>

    <div v-else class="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 uppercase text-xs">
          <tr>
            <th class="text-left px-4 py-3">Status</th>
            <th class="text-left px-4 py-3">Task</th>
            <th class="text-left px-4 py-3">Time</th>
            <th class="text-left px-4 py-3">Duration</th>
            <th class="text-left px-4 py-3">Articles</th>
            <th class="text-left px-4 py-3">Error</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <tr v-for="entry in store.history" :key="entry.executed_at" class="hover:bg-gray-50">
            <td class="px-4 py-3">
              <span :class="entry.status === 'Success' ? 'text-green-500' : 'text-red-500'" class="text-lg">
                {{ entry.status === 'Success' ? '✅' : '❌' }}
              </span>
            </td>
            <td class="px-4 py-3 font-medium text-gray-700">{{ entry.task_name }}</td>
            <td class="px-4 py-3 text-gray-500">{{ formatTime(entry.executed_at) }}</td>
            <td class="px-4 py-3 text-gray-500">{{ formatDuration(entry.duration_ms) }}</td>
            <td class="px-4 py-3 text-gray-500">{{ entry.articles_count }}</td>
            <td class="px-4 py-3">
              <span v-if="entry.error_message" class="text-red-500 text-xs">
                {{ entry.error_message }}
              </span>
              <span v-else class="text-gray-300">—</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
