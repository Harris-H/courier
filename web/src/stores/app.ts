import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getStatus, getTasks, getHistory, getConfig, type StatusResponse, type TaskInfo, type HistoryEntry, type ConfigOverview } from '../api'

export const useAppStore = defineStore('app', () => {
  const status = ref<StatusResponse | null>(null)
  const tasks = ref<TaskInfo[]>([])
  const history = ref<HistoryEntry[]>([])
  const config = ref<ConfigOverview | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchStatus() {
    try {
      const res = await getStatus()
      status.value = res.data
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : '获取状态失败'
    }
  }

  async function fetchTasks() {
    try {
      const res = await getTasks()
      tasks.value = res.data
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : '获取任务列表失败'
    }
  }

  async function fetchHistory() {
    try {
      const res = await getHistory()
      history.value = res.data
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : '获取执行历史失败'
    }
  }

  async function fetchConfig() {
    try {
      const res = await getConfig()
      config.value = res.data
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : '获取配置失败'
    }
  }

  async function fetchAll() {
    loading.value = true
    error.value = null
    await Promise.all([fetchStatus(), fetchTasks(), fetchHistory(), fetchConfig()])
    loading.value = false
  }

  return { status, tasks, history, config, loading, error, fetchStatus, fetchTasks, fetchHistory, fetchConfig, fetchAll }
})
