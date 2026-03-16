<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import { getHistoryContent } from '../api'

const store = useAppStore()
const expandedIndex = ref<number | null>(null)
const contentCache = ref<Record<number, string>>({})
const loadingContent = ref(false)

onMounted(() => store.fetchHistory())

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleString()
}

async function toggleContent(index: number) {
  if (expandedIndex.value === index) {
    expandedIndex.value = null
    return
  }

  expandedIndex.value = index

  if (contentCache.value[index]) return

  loadingContent.value = true
  try {
    const res = await getHistoryContent(index)
    if (res.data.content) {
      contentCache.value[index] = res.data.content
    }
  } catch {
    contentCache.value[index] = '加载失败'
  } finally {
    loadingContent.value = false
  }
}
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-2xl font-bold text-gray-800">执行历史</h2>
      <button
        @click="store.fetchHistory()"
        class="px-3 py-1.5 text-sm bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 transition-colors cursor-pointer"
      >
        🔄 刷新
      </button>
    </div>

    <div v-if="store.history.length === 0" class="text-gray-400 text-center py-12">
      暂无执行记录，任务运行后将显示在此。
    </div>

    <div v-else class="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 uppercase text-xs">
          <tr>
            <th class="text-left px-4 py-3">状态</th>
            <th class="text-left px-4 py-3">任务</th>
            <th class="text-left px-4 py-3">时间</th>
            <th class="text-left px-4 py-3">耗时</th>
            <th class="text-left px-4 py-3">文章数</th>
            <th class="text-left px-4 py-3">错误</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <template v-for="(entry, index) in store.history" :key="entry.executed_at">
            <tr class="hover:bg-gray-50">
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
                <button
                  v-else-if="entry.has_content"
                  @click="toggleContent(index)"
                  class="text-blue-500 hover:text-blue-700 text-xs cursor-pointer underline"
                >
                  {{ expandedIndex === index ? '收起' : '查看内容' }}
                </button>
                <span v-else class="text-gray-300">—</span>
              </td>
            </tr>
            <!-- Expanded content row -->
            <tr v-if="expandedIndex === index">
              <td colspan="6" class="px-4 py-4 bg-gray-50">
                <div v-if="loadingContent" class="text-gray-400 text-sm">加载中...</div>
                <div v-else-if="contentCache[index]" class="text-sm text-gray-700 whitespace-pre-wrap font-mono leading-relaxed max-h-96 overflow-y-auto">
                  {{ contentCache[index] }}
                </div>
                <div v-else class="text-gray-400 text-sm">暂无内容</div>
              </td>
            </tr>
          </template>
        </tbody>
      </table>
    </div>
  </div>
</template>
