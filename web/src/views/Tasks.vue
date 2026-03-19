<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { runTask, updateTaskSchedule, toggleTask } from '../api'

const store = useAppStore()
const toast = useToastStore()
const runningTask = ref<string | null>(null)
const editingTask = ref<string | null>(null)
const editName = ref('')
const editCron = ref('')
const editRetries = ref(2)
const editChannels = ref<string[]>([])
const editErrors = ref<Record<string, string>>({})

onMounted(() => store.fetchTasks())

function validateTaskForm(): boolean {
  editErrors.value = {}

  if (!editName.value.trim()) {
    editErrors.value.name = '任务名称不能为空'
  } else if (editName.value.trim().length > 50) {
    editErrors.value.name = '任务名称不能超过 50 个字符'
  }

  if (!editCron.value.trim()) {
    editErrors.value.cron = 'Cron 表达式不能为空'
  } else {
    const parts = editCron.value.trim().split(/\s+/)
    if (parts.length < 5 || parts.length > 7) {
      editErrors.value.cron = 'Cron 表达式格式无效（需要 5-7 个字段）'
    }
  }

  if (editRetries.value < 0 || editRetries.value > 10 || !Number.isInteger(editRetries.value)) {
    editErrors.value.retries = '重试次数需为 0-10 的整数'
  }

  return Object.keys(editErrors.value).length === 0
}

async function handleRunTask(name: string) {
  runningTask.value = name
  try {
    const res = await runTask(name)
    toast.success(res.data.message)
    setTimeout(() => store.fetchHistory(), 2000)
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '未知错误'
    toast.error(`任务执行失败：${msg}`)
  } finally {
    runningTask.value = null
  }
}

const availableChannels = ['feishu', 'telegram', 'email']

function startEdit(name: string, currentCron: string, currentRetries: number, currentChannels: string[]) {
  editingTask.value = name
  editName.value = name
  editCron.value = currentCron
  editRetries.value = currentRetries
  editChannels.value = [...currentChannels]
  editErrors.value = {}
}

async function saveEdit(originalName: string) {
  if (!validateTaskForm()) {
    Object.values(editErrors.value).forEach(msg => toast.error(msg))
    return
  }

  try {
    const res = await updateTaskSchedule(originalName, {
      name: editName.value !== originalName ? editName.value : undefined,
      cron: editCron.value,
      max_retries: editRetries.value,
      channels: editChannels.value,
    })
    if (res.data.success) {
      toast.success(res.data.message)
      editingTask.value = null
      await store.fetchTasks()
    } else {
      toast.error(res.data.message)
    }
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '未知错误'
    toast.error(`保存失败：${msg}`)
  }
}

async function handleToggle(name: string, currentEnabled: boolean) {
  try {
    const res = await toggleTask(name, { enabled: !currentEnabled })
    if (res.data.success) {
      toast.success(res.data.message)
      await store.fetchTasks()
    } else {
      toast.error(res.data.message)
    }
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '未知错误'
    toast.error(`切换失败：${msg}`)
  }
}

function cancelEdit() {
  editingTask.value = null
  editErrors.value = {}
}
</script>

<template>
  <div>
    <h2 class="text-2xl font-bold text-gray-800 mb-4">定时任务</h2>

    <div v-if="store.tasks.length === 0" class="text-gray-400 text-center py-12">
      未配置任何任务，请在 config.toml 中添加计划。
    </div>

    <div v-else class="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden divide-y divide-gray-100">
      <div
        v-for="task in store.tasks"
        :key="task.name"
        :class="[task.enabled ? '' : 'opacity-50']"
      >
        <!-- Display Mode -->
        <template v-if="editingTask !== task.name">
          <div class="flex items-center gap-3 px-4 py-3">
            <!-- Toggle -->
            <button
              @click="handleToggle(task.name, task.enabled)"
              :class="[
                'relative inline-flex h-5 w-9 flex-shrink-0 items-center rounded-full transition-colors cursor-pointer',
                task.enabled ? 'bg-blue-600' : 'bg-gray-300'
              ]"
              :title="task.enabled ? '点击禁用' : '点击启用'"
            >
              <span
                :class="[
                  'inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform',
                  task.enabled ? 'translate-x-[18px]' : 'translate-x-[3px]'
                ]"
              />
            </button>

            <!-- Task info -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="font-semibold text-sm text-gray-800 truncate">{{ task.name }}</span>
                <code class="text-xs text-gray-400 bg-gray-50 px-1.5 py-0.5 rounded font-mono">{{ task.cron }}</code>
                <span v-for="s in task.sources" :key="s"
                  class="inline-block bg-blue-50 text-blue-600 px-1.5 py-0.5 rounded text-[11px] leading-tight"
                >{{ s }}</span>
                <span v-for="c in task.channels" :key="c"
                  class="inline-block bg-green-50 text-green-600 px-1.5 py-0.5 rounded text-[11px] leading-tight"
                >{{ c }}</span>
              </div>
            </div>

            <!-- Actions -->
            <div class="flex items-center gap-1.5 flex-shrink-0">
              <button
                @click="startEdit(task.name, task.cron, task.max_retries, task.channels)"
                class="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-md transition-colors cursor-pointer"
                title="编辑任务"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                </svg>
              </button>
              <button
                @click="handleRunTask(task.name)"
                :disabled="runningTask === task.name"
                :class="[
                  'px-3 py-1 text-xs font-medium rounded-md transition-colors cursor-pointer',
                  runningTask === task.name
                    ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
                    : 'bg-blue-600 text-white hover:bg-blue-700'
                ]"
              >
                {{ runningTask === task.name ? '运行中...' : '▶ 运行' }}
              </button>
            </div>
          </div>
        </template>

        <!-- Edit Mode -->
        <template v-else>
          <div class="px-4 py-3 bg-blue-50/30">
            <div class="flex items-center justify-between mb-3">
              <span class="text-sm font-semibold text-blue-600">编辑任务</span>
              <div class="flex items-center gap-1.5">
                <button @click="saveEdit(task.name)"
                  class="px-3 py-1 text-xs font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors cursor-pointer">
                  保存
                </button>
                <button @click="cancelEdit"
                  class="px-3 py-1 text-xs font-medium bg-white text-gray-600 border border-gray-200 rounded-md hover:bg-gray-50 transition-colors cursor-pointer">
                  取消
                </button>
              </div>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <div>
                <label class="block text-xs text-gray-500 mb-1">任务名称</label>
                <input
                  v-model="editName"
                  type="text"
                  :class="['w-full px-2.5 py-1.5 text-sm border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500', editErrors.name ? 'border-red-400' : 'border-gray-300']"
                  placeholder="输入任务名称"
                />
                <p v-if="editErrors.name" class="text-xs text-red-500 mt-1">{{ editErrors.name }}</p>
              </div>
              <div>
                <label class="block text-xs text-gray-500 mb-1">Cron 表达式</label>
                <input
                  v-model="editCron"
                  type="text"
                  :class="['w-full px-2.5 py-1.5 text-sm font-mono border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500', editErrors.cron ? 'border-red-400' : 'border-gray-300']"
                  placeholder="0 0 10 * * *"
                />
                <p v-if="editErrors.cron" class="text-xs text-red-500 mt-1">{{ editErrors.cron }}</p>
              </div>
              <div>
                <label class="block text-xs text-gray-500 mb-1">最大重试次数</label>
                <input
                  v-model.number="editRetries"
                  type="number"
                  min="0"
                  max="10"
                  :class="['w-full px-2.5 py-1.5 text-sm border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500', editErrors.retries ? 'border-red-400' : 'border-gray-300']"
                />
                <p v-if="editErrors.retries" class="text-xs text-red-500 mt-1">{{ editErrors.retries }}</p>
              </div>
            </div>

            <div class="flex flex-wrap gap-3 text-xs mt-3 pt-2 border-t border-blue-100/50">
              <div>
                <span class="text-gray-400">数据源：</span>
                <span v-for="s in task.sources" :key="s"
                  class="inline-block bg-blue-50 text-blue-600 px-1.5 py-0.5 rounded text-[11px] mr-1"
                >{{ s }}</span>
              </div>
              <div class="flex items-center gap-2">
                <span class="text-gray-400">推送渠道：</span>
                <label v-for="ch in availableChannels" :key="ch" class="inline-flex items-center gap-1 cursor-pointer">
                  <input type="checkbox" :value="ch" v-model="editChannels"
                    class="w-3.5 h-3.5 rounded border-gray-300 text-blue-600 focus:ring-blue-500 cursor-pointer" />
                  <span class="text-[11px] text-gray-600">{{ ch }}</span>
                </label>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
