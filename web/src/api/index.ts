import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  timeout: 15000,
})

export interface StatusResponse {
  version: string
  uptime_secs: number
  tasks_count: number
  sources_count: number
  channels_count: number
}

export interface TaskInfo {
  name: string
  cron: string
  sources: string[]
  channels: string[]
  run_on_start: boolean
  max_retries: number
}

export interface HistoryEntry {
  task_name: string
  status: string
  executed_at: string
  duration_ms: number
  articles_count: number
  error_message: string | null
}

export interface ConfigOverview {
  log_level: string
  llm_model: string
  llm_api_base: string
  sources: { hackernews: boolean; reddit: boolean; rss: boolean }
  channels: { telegram: boolean; feishu: boolean; email: boolean }
}

export interface SourceInfo {
  name: string
  enabled: boolean
}

export const getStatus = () => api.get<StatusResponse>('/status')
export const getTasks = () => api.get<TaskInfo[]>('/tasks')
export const runTask = (name: string) => api.post(`/tasks/${name}/run`)
export const getHistory = () => api.get<HistoryEntry[]>('/history')
export const getConfig = () => api.get<ConfigOverview>('/config')
export const getSources = () => api.get<SourceInfo[]>('/sources')

export default api
