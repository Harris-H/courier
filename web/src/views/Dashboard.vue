<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { useAppStore } from '../stores/app'

const store = useAppStore()

onMounted(() => store.fetchAll())

const uptimeText = computed(() => {
  if (!store.status) return '--'
  const secs = store.status.uptime_secs
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = secs % 60
  return `${h}h ${m}m ${s}s`
})

const recentHistory = computed(() => store.history.slice(0, 5))

const successRate = computed(() => {
  if (store.history.length === 0) return '--'
  const ok = store.history.filter(h => h.status === 'Success').length
  return `${Math.round((ok / store.history.length) * 100)}%`
})
</script>

<template>
  <div>
    <h2 class="text-2xl font-bold text-gray-800 mb-6">仪表盘</h2>

    <!-- Loading -->
    <div v-if="store.loading" class="text-gray-400 text-center py-12">加载中...</div>

    <!-- Error -->
    <div v-else-if="store.error" class="bg-red-50 text-red-600 p-4 rounded-lg">
      ❌ {{ store.error }}
    </div>

    <template v-else>
      <!-- Stats Cards -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <p class="text-xs text-gray-400 uppercase tracking-wide">版本</p>
          <p class="text-2xl font-bold text-gray-800 mt-1">v{{ store.status?.version }}</p>
        </div>
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <p class="text-xs text-gray-400 uppercase tracking-wide">运行时间</p>
          <p class="text-2xl font-bold text-gray-800 mt-1">{{ uptimeText }}</p>
        </div>
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <p class="text-xs text-gray-400 uppercase tracking-wide">任务数</p>
          <p class="text-2xl font-bold text-gray-800 mt-1">{{ store.status?.tasks_count ?? 0 }}</p>
        </div>
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <p class="text-xs text-gray-400 uppercase tracking-wide">成功率</p>
          <p class="text-2xl font-bold text-green-600 mt-1">{{ successRate }}</p>
        </div>
      </div>

      <!-- Overview -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- Sources & Channels -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">活跃组件</h3>
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-gray-700">📰 数据源</span>
              <span class="text-sm font-medium text-gray-500">{{ store.status?.sources_count ?? 0 }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-gray-700">📤 推送渠道</span>
              <span class="text-sm font-medium text-gray-500">{{ store.status?.channels_count ?? 0 }}</span>
            </div>
          </div>
        </div>

        <!-- Recent Executions -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">最近执行</h3>
          <div v-if="recentHistory.length === 0" class="text-gray-400 text-sm">暂无执行记录</div>
          <div v-else class="space-y-2">
            <div
              v-for="entry in recentHistory"
              :key="entry.executed_at"
              class="flex items-center justify-between text-sm"
            >
              <div class="flex items-center gap-2">
                <span :class="entry.status === 'Success' ? 'text-green-500' : 'text-red-500'">
                  {{ entry.status === 'Success' ? '✅' : '❌' }}
                </span>
                <span class="text-gray-700">{{ entry.task_name }}</span>
              </div>
              <span class="text-gray-400 text-xs">{{ new Date(entry.executed_at).toLocaleString() }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
