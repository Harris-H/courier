import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  timeout: 15000,
})

// Global response error interceptor
api.interceptors.response.use(
  response => response,
  error => {
    if (error.response?.status === 401) {
      console.error('[Courier] 认证失败：API Key 无效或未配置')
    } else if (error.response?.status >= 500) {
      console.error('[Courier] 服务器错误:', error.response?.status)
    } else if (!error.response) {
      console.error('[Courier] 网络错误：无法连接到后端服务')
    }
    return Promise.reject(error)
  }
)

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
  enabled: boolean
}

export interface HistoryEntry {
  task_name: string
  status: string
  executed_at: string
  duration_ms: number
  articles_count: number
  error_message: string | null
  has_content: boolean
}

export interface HistoryContentResponse {
  content: string | null
}

export interface ConfigOverview {
  log_level: string
  llm_model: string
  llm_api_base: string
  llm_max_tokens: number
  sources: { hackernews: boolean; reddit: boolean; rss: boolean }
  channels: { telegram: boolean; feishu: boolean; email: boolean }
  feishu_webhook_url: string
  email_config: EmailConfigOverview
}

export interface EmailConfigOverview {
  enabled: boolean
  smtp_host: string
  smtp_port: number
  smtp_username: string
  has_password: boolean
  from: string
  to: string[]
}

export interface SourceInfo {
  name: string
  enabled: boolean
}

export interface FeishuConfigUpdate {
  enabled: boolean
  webhook_url: string
}

export interface EmailConfigUpdate {
  enabled: boolean
  smtp_host: string
  smtp_port: number
  smtp_username: string
  smtp_password: string
  from: string
  to: string[]
}

export interface LlmConfigUpdate {
  model: string
  max_tokens?: number
}

export interface UpdateConfigResponse {
  success: boolean
  message: string
}

export interface UpdateScheduleRequest {
  name?: string
  cron?: string
  max_retries?: number
  channels?: string[]
}

export interface ToggleTaskRequest {
  enabled: boolean
}

export const getStatus = () => api.get<StatusResponse>('/status')
export const getTasks = () => api.get<TaskInfo[]>('/tasks')
export const runTask = (name: string) => api.post(`/tasks/${name}/run`)
export const updateTaskSchedule = (name: string, data: UpdateScheduleRequest) =>
  api.put<UpdateConfigResponse>(`/tasks/${name}/schedule`, data)
export const toggleTask = (name: string, data: ToggleTaskRequest) =>
  api.put<UpdateConfigResponse>(`/tasks/${name}/toggle`, data)
export const getHistory = () => api.get<HistoryEntry[]>('/history')
export const getHistoryContent = (index: number) =>
  api.get<HistoryContentResponse>(`/history/${index}/content`)
export const deleteHistoryBatch = (timestamps: string[]) =>
  api.post<UpdateConfigResponse>('/history/batch', { timestamps })
export const clearAllHistory = () =>
  api.delete<UpdateConfigResponse>('/history/clear')
export const getConfig = () => api.get<ConfigOverview>('/config')
export const getSources = () => api.get<SourceInfo[]>('/sources')
export const updateFeishuConfig = (data: FeishuConfigUpdate) =>
  api.put<UpdateConfigResponse>('/config/feishu', data)
export const updateEmailConfig = (data: EmailConfigUpdate) =>
  api.put<UpdateConfigResponse>('/config/email', data)
export const updateLlmConfig = (data: LlmConfigUpdate) =>
  api.put<UpdateConfigResponse>('/config/llm', data)

export default api
