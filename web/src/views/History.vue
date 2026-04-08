<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { getHistoryContent } from '../api'

const store = useAppStore()
const expandedIndex = ref<number | null>(null)
const contentCache = ref<Record<number, string>>({})
const loadingContent = ref(false)
const selectedItems = ref<Set<string>>(new Set())
const showClearConfirm = ref(false)
const deleting = ref(false)
const currentPage = ref(1)
const pageSize = ref(15)
let pollTimer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  store.fetchHistory()
  startPollingIfNeeded()
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})

// Auto-refresh when there are Running tasks
const hasRunningTasks = computed(() =>
  store.history.some(e => e.status === 'Running')
)

watch(hasRunningTasks, (val) => {
  if (val) {
    startPollingIfNeeded()
  } else if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
})

function startPollingIfNeeded() {
  if (pollTimer) return
  pollTimer = setInterval(() => {
    if (hasRunningTasks.value) {
      store.fetchHistory()
    } else if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }, 3000)
}

// Pagination
const totalPages = computed(() => Math.max(1, Math.ceil(store.history.length / pageSize.value)))
const paginatedHistory = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return store.history.slice(start, start + pageSize.value).map((entry, i) => ({
    ...entry,
    globalIndex: start + i, // keep original index for content API
  }))
})

// Reset to page 1 when history changes significantly
watch(() => store.history.length, () => {
  if (currentPage.value > totalPages.value) {
    currentPage.value = totalPages.value
  }
})

function goToPage(page: number) {
  currentPage.value = Math.max(1, Math.min(page, totalPages.value))
  expandedIndex.value = null
}

const visiblePages = computed(() => {
  const total = totalPages.value
  const current = currentPage.value
  const pages: (number | '...')[] = []
  if (total <= 7) {
    for (let i = 1; i <= total; i++) pages.push(i)
  } else {
    pages.push(1)
    if (current > 3) pages.push('...')
    for (let i = Math.max(2, current - 1); i <= Math.min(total - 1, current + 1); i++) {
      pages.push(i)
    }
    if (current < total - 2) pages.push('...')
    pages.push(total)
  }
  return pages
})

const allSelected = computed(() =>
  paginatedHistory.value.length > 0 && paginatedHistory.value.every(e => selectedItems.value.has(e.executed_at))
)

function toggleSelectAll() {
  if (allSelected.value) {
    for (const e of paginatedHistory.value) {
      selectedItems.value.delete(e.executed_at)
    }
    selectedItems.value = new Set(selectedItems.value)
  } else {
    selectedItems.value = new Set([...selectedItems.value, ...paginatedHistory.value.map(e => e.executed_at)])
  }
}

function toggleSelect(executedAt: string) {
  const s = new Set(selectedItems.value)
  if (s.has(executedAt)) {
    s.delete(executedAt)
  } else {
    s.add(executedAt)
  }
  selectedItems.value = s
}

async function deleteSelected() {
  if (selectedItems.value.size === 0) return
  deleting.value = true
  await store.deleteHistoryItems([...selectedItems.value])
  selectedItems.value = new Set()
  expandedIndex.value = null
  contentCache.value = {}
  deleting.value = false
}

async function clearAll() {
  deleting.value = true
  showClearConfirm.value = false
  await store.clearHistory()
  selectedItems.value = new Set()
  expandedIndex.value = null
  contentCache.value = {}
  deleting.value = false
}

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
      <div class="flex items-center gap-2">
        <button
          v-if="selectedItems.size > 0"
          @click="deleteSelected"
          :disabled="deleting"
          class="px-3 py-1.5 text-sm bg-red-50 text-red-600 rounded-lg hover:bg-red-100 transition-colors cursor-pointer disabled:opacity-50"
        >
          🗑️ 删除选中 ({{ selectedItems.size }})
        </button>
        <button
          v-if="store.history.length > 0"
          @click="showClearConfirm = true"
          :disabled="deleting"
          class="px-3 py-1.5 text-sm bg-red-50 text-red-600 rounded-lg hover:bg-red-100 transition-colors cursor-pointer disabled:opacity-50"
        >
          🧹 清空全部
        </button>
        <button
          @click="store.fetchHistory()"
          class="px-3 py-1.5 text-sm bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 transition-colors cursor-pointer"
        >
          🔄 刷新
        </button>
      </div>
    </div>

    <!-- Clear all confirmation dialog -->
    <div v-if="showClearConfirm" class="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
      <div class="bg-white rounded-xl p-6 shadow-lg max-w-sm mx-4">
        <h3 class="text-lg font-semibold text-gray-800 mb-2">确认清空</h3>
        <p class="text-gray-600 text-sm mb-4">确定要清空所有执行历史记录吗？此操作不可恢复。</p>
        <div class="flex justify-end gap-2">
          <button
            @click="showClearConfirm = false"
            class="px-4 py-2 text-sm bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 cursor-pointer"
          >
            取消
          </button>
          <button
            @click="clearAll"
            class="px-4 py-2 text-sm bg-red-500 text-white rounded-lg hover:bg-red-600 cursor-pointer"
          >
            确认清空
          </button>
        </div>
      </div>
    </div>

    <div v-if="store.history.length === 0" class="text-gray-400 text-center py-12">
      暂无执行记录，任务运行后将显示在此。
    </div>

    <div v-else class="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 uppercase text-xs">
          <tr>
            <th class="text-left px-4 py-3 w-10">
              <input
                type="checkbox"
                :checked="allSelected"
                @change="toggleSelectAll"
                class="rounded border-gray-300 cursor-pointer"
              />
            </th>
            <th class="text-left px-4 py-3">状态</th>
            <th class="text-left px-4 py-3">任务</th>
            <th class="text-left px-4 py-3">开始时间</th>
            <th class="text-left px-4 py-3">完成时间</th>
            <th class="text-left px-4 py-3">耗时</th>
            <th class="text-left px-4 py-3">文章数</th>
            <th class="text-left px-4 py-3">操作</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <template v-for="entry in paginatedHistory" :key="entry.executed_at">
            <tr class="hover:bg-gray-50">
              <td class="px-4 py-3">
                <input
                  type="checkbox"
                  :checked="selectedItems.has(entry.executed_at)"
                  @change="toggleSelect(entry.executed_at)"
                  class="rounded border-gray-300 cursor-pointer"
                />
              </td>
              <td class="px-4 py-3">
                <span v-if="entry.status === 'Running'" class="text-lg inline-flex items-center" title="执行中">
                  <svg class="animate-spin h-5 w-5 text-blue-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                  </svg>
                </span>
                <span v-else :class="entry.status === 'Success' ? 'text-green-500' : 'text-red-500'" class="text-lg">
                  {{ entry.status === 'Success' ? '✅' : '❌' }}
                </span>
              </td>
              <td class="px-4 py-3 font-medium text-gray-700">{{ entry.task_name }}</td>
              <td class="px-4 py-3 text-gray-500">{{ formatTime(entry.executed_at) }}</td>
              <td class="px-4 py-3 text-gray-500">
                <span v-if="entry.status === 'Running'" class="text-blue-500 text-xs">—</span>
                <span v-else-if="entry.completed_at">{{ formatTime(entry.completed_at) }}</span>
                <span v-else class="text-gray-300">—</span>
              </td>
              <td class="px-4 py-3 text-gray-500">
                <span v-if="entry.status === 'Running'" class="text-blue-500 text-xs">执行中...</span>
                <span v-else>{{ formatDuration(entry.duration_ms) }}</span>
              </td>
              <td class="px-4 py-3 text-gray-500">
                <span v-if="entry.status === 'Running'" class="text-blue-500 text-xs">—</span>
                <span v-else>{{ entry.articles_count }}</span>
              </td>
              <td class="px-4 py-3">
                <span v-if="entry.error_message" class="text-red-500 text-xs">
                  {{ entry.error_message }}
                </span>
                <button
                  v-else-if="entry.has_content"
                  @click="toggleContent(entry.globalIndex)"
                  class="text-blue-500 hover:text-blue-700 text-xs cursor-pointer underline"
                >
                  {{ expandedIndex === entry.globalIndex ? '收起' : '查看内容' }}
                </button>
                <span v-else class="text-gray-300">—</span>
              </td>
            </tr>
            <!-- Expanded content row -->
            <tr v-if="expandedIndex === entry.globalIndex">
              <td colspan="8" class="px-4 py-4 bg-gray-50">
                <div v-if="loadingContent" class="text-gray-400 text-sm">加载中...</div>
                <div v-else-if="contentCache[entry.globalIndex]" class="text-sm text-gray-700 whitespace-pre-wrap font-mono leading-relaxed max-h-96 overflow-y-auto">
                  {{ contentCache[entry.globalIndex] }}
                </div>
                <div v-else class="text-gray-400 text-sm">暂无内容</div>
              </td>
            </tr>
          </template>
        </tbody>
      </table>

      <!-- Pagination -->
      <div v-if="totalPages > 1" class="flex items-center justify-between px-4 py-3 border-t border-gray-200 bg-gray-50">
        <span class="text-xs text-gray-500">
          共 {{ store.history.length }} 条，第 {{ currentPage }}/{{ totalPages }} 页
        </span>
        <div class="flex items-center gap-1">
          <button
            @click="goToPage(currentPage - 1)"
            :disabled="currentPage <= 1"
            class="px-2 py-1 text-xs rounded border border-gray-300 hover:bg-gray-100 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
          >
            ‹
          </button>
          <template v-for="p in visiblePages" :key="p">
            <span v-if="p === '...'" class="px-1 text-xs text-gray-400">…</span>
            <button
              v-else
              @click="goToPage(p as number)"
              :class="p === currentPage ? 'bg-blue-500 text-white border-blue-500' : 'border-gray-300 hover:bg-gray-100'"
              class="px-2.5 py-1 text-xs rounded border cursor-pointer"
            >
              {{ p }}
            </button>
          </template>
          <button
            @click="goToPage(currentPage + 1)"
            :disabled="currentPage >= totalPages"
            class="px-2 py-1 text-xs rounded border border-gray-300 hover:bg-gray-100 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
          >
            ›
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
