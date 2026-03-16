<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import { runTask } from '../api'

const store = useAppStore()
const runningTask = ref<string | null>(null)
const message = ref<string | null>(null)

onMounted(() => store.fetchTasks())

async function handleRunTask(name: string) {
  runningTask.value = name
  message.value = null
  try {
    const res = await runTask(name)
    message.value = `✅ ${res.data.message}`
    // Refresh history after a short delay
    setTimeout(() => store.fetchHistory(), 2000)
  } catch (e: any) {
    message.value = `❌ Failed: ${e.message}`
  } finally {
    runningTask.value = null
  }
}
</script>

<template>
  <div>
    <h2 class="text-2xl font-bold text-gray-800 mb-6">Scheduled Tasks</h2>

    <!-- Feedback -->
    <div v-if="message" class="mb-4 p-3 rounded-lg text-sm"
      :class="message.startsWith('✅') ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'"
    >
      {{ message }}
    </div>

    <div v-if="store.tasks.length === 0" class="text-gray-400 text-center py-12">
      No tasks configured. Add schedules to config.toml.
    </div>

    <div v-else class="space-y-4">
      <div
        v-for="task in store.tasks"
        :key="task.name"
        class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm"
      >
        <div class="flex items-start justify-between">
          <div>
            <h3 class="text-lg font-semibold text-gray-800">{{ task.name }}</h3>
            <p class="text-sm text-gray-400 mt-1 font-mono">cron: {{ task.cron }}</p>
          </div>
          <button
            @click="handleRunTask(task.name)"
            :disabled="runningTask === task.name"
            class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
          >
            {{ runningTask === task.name ? 'Running...' : '▶ Run Now' }}
          </button>
        </div>

        <div class="mt-4 flex flex-wrap gap-4 text-sm">
          <div>
            <span class="text-gray-400">Sources: </span>
            <span v-for="s in task.sources" :key="s"
              class="inline-block bg-blue-50 text-blue-700 px-2 py-0.5 rounded text-xs mr-1"
            >{{ s }}</span>
          </div>
          <div>
            <span class="text-gray-400">Channels: </span>
            <span v-for="c in task.channels" :key="c"
              class="inline-block bg-green-50 text-green-700 px-2 py-0.5 rounded text-xs mr-1"
            >{{ c }}</span>
          </div>
          <div class="text-gray-400">
            Retries: {{ task.max_retries }} | Auto-start: {{ task.run_on_start ? 'Yes' : 'No' }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
