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
    } catch (e: any) {
      error.value = e.message
    }
  }

  async function fetchTasks() {
    try {
      const res = await getTasks()
      tasks.value = res.data
    } catch (e: any) {
      error.value = e.message
    }
  }

  async function fetchHistory() {
    try {
      const res = await getHistory()
      history.value = res.data
    } catch (e: any) {
      error.value = e.message
    }
  }

  async function fetchConfig() {
    try {
      const res = await getConfig()
      config.value = res.data
    } catch (e: any) {
      error.value = e.message
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
