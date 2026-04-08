<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { use } from 'echarts/core'
import { BarChart } from 'echarts/charts'
import {
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent,
} from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import { useAppStore } from '../stores/app'

use([BarChart, TitleComponent, TooltipComponent, LegendComponent, GridComponent, CanvasRenderer])

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
  const finished = store.history.filter(h => h.status !== 'Running')
  if (finished.length === 0) return '--'
  const ok = finished.filter(h => h.status === 'Success').length
  return `${Math.round((ok / finished.length) * 100)}%`
})

// Aggregate stats per task
interface TaskStats {
  name: string
  total: number
  success: number
  failed: number
  avgDuration: number
  successRate: number
}

const taskStats = computed<TaskStats[]>(() => {
  const map = new Map<string, { total: number; success: number; failed: number; totalDuration: number }>()
  for (const h of store.history) {
    if (h.status === 'Running') continue
    const entry = map.get(h.task_name) ?? { total: 0, success: 0, failed: 0, totalDuration: 0 }
    entry.total++
    if (h.status === 'Success') entry.success++
    else entry.failed++
    entry.totalDuration += h.duration_ms ?? 0
    map.set(h.task_name, entry)
  }
  return Array.from(map.entries()).map(([name, s]) => ({
    name,
    total: s.total,
    success: s.success,
    failed: s.failed,
    avgDuration: s.total > 0 ? Math.round(s.totalDuration / s.total / 1000) : 0,
    successRate: s.total > 0 ? Math.round((s.success / s.total) * 100) : 0,
  }))
})

// Chart 1: Execution count per task (stacked bar)
const executionCountOption = computed(() => ({
  tooltip: {
    trigger: 'axis',
    axisPointer: { type: 'shadow' },
  },
  legend: { data: ['成功', '失败'], bottom: 0 },
  grid: { left: 16, right: 16, top: 16, bottom: 40, containLabel: true },
  xAxis: {
    type: 'category',
    data: taskStats.value.map(s => s.name),
    axisLabel: { rotate: taskStats.value.length > 4 ? 20 : 0, fontSize: 11 },
  },
  yAxis: { type: 'value', minInterval: 1 },
  series: [
    {
      name: '成功',
      type: 'bar',
      stack: 'total',
      data: taskStats.value.map(s => s.success),
      itemStyle: { color: '#22c55e', borderRadius: [0, 0, 0, 0] },
    },
    {
      name: '失败',
      type: 'bar',
      stack: 'total',
      data: taskStats.value.map(s => s.failed),
      itemStyle: { color: '#ef4444', borderRadius: [4, 4, 0, 0] },
    },
  ],
}))

// Chart 2: Average duration per task (bar)
const avgDurationOption = computed(() => ({
  tooltip: {
    trigger: 'axis',
    axisPointer: { type: 'shadow' },
    formatter: (params: any) => {
      const p = params[0]
      return `${p.name}<br/>${p.seriesName}: ${p.value}s`
    },
  },
  grid: { left: 16, right: 16, top: 16, bottom: 8, containLabel: true },
  xAxis: {
    type: 'category',
    data: taskStats.value.map(s => s.name),
    axisLabel: { rotate: taskStats.value.length > 4 ? 20 : 0, fontSize: 11 },
  },
  yAxis: { type: 'value', axisLabel: { formatter: '{value}s' } },
  series: [
    {
      name: '平均耗时',
      type: 'bar',
      data: taskStats.value.map(s => s.avgDuration),
      itemStyle: {
        color: {
          type: 'linear',
          x: 0, y: 0, x2: 0, y2: 1,
          colorStops: [
            { offset: 0, color: '#6366f1' },
            { offset: 1, color: '#a5b4fc' },
          ],
        },
        borderRadius: [4, 4, 0, 0],
      },
    },
  ],
}))
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

      <!-- Charts Row -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8" v-if="taskStats.length > 0">
        <!-- Execution Count -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-3">执行次数</h3>
          <v-chart :option="executionCountOption" autoresize style="height: 240px" />
        </div>

        <!-- Average Duration -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-3">平均耗时</h3>
          <v-chart :option="avgDurationOption" autoresize style="height: 240px" />
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
                <span :class="entry.status === 'Success' ? 'text-green-500' : entry.status === 'Running' ? 'text-blue-500' : 'text-red-500'">
                  {{ entry.status === 'Success' ? '✅' : entry.status === 'Running' ? '🔄' : '❌' }}
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
