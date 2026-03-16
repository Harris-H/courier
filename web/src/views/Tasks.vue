<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { runTask, updateTaskSchedule } from '../api'

const store = useAppStore()
const toast = useToastStore()
const runningTask = ref<string | null>(null)
const editingTask = ref<string | null>(null)
const editName = ref('')
const editCron = ref('')
const editRetries = ref(2)

onMounted(() => store.fetchTasks())

async function handleRunTask(name: string) {
  runningTask.value = name
  try {
    const res = await runTask(name)
    toast.success(res.data.message)
    setTimeout(() => store.fetchHistory(), 2000)
  } catch (e: any) {
    toast.error(`任务执行失败：${e.message}`)
  } finally {
    runningTask.value = null
  }
}

function startEdit(name: string, currentCron: string, currentRetries: number) {
  editingTask.value = name
  editName.value = name
  editCron.value = currentCron
  editRetries.value = currentRetries
}

async function saveEdit(originalName: string) {
  try {
    const res = await updateTaskSchedule(originalName, {
      name: editName.value !== originalName ? editName.value : undefined,
      cron: editCron.value,
      max_retries: editRetries.value,
    })
    if (res.data.success) {
      toast.success(res.data.message)
      editingTask.value = null
      await store.fetchTasks()
    } else {
      toast.error(res.data.message)
    }
  } catch (e: any) {
    toast.error(`保存失败：${e.message}`)
  }
}

function cancelEdit() {
  editingTask.value = null
}
</script>

<template>
  <div>
    <h2 class="text-2xl font-bold text-gray-800 mb-6">定时任务</h2>

    <div v-if="store.tasks.length === 0" class="text-gray-400 text-center py-12">
      未配置任何任务，请在 config.toml 中添加计划。
    </div>

    <div v-else class="space-y-4">
      <div
        v-for="task in store.tasks"
        :key="task.name"
        class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm"
      >
        <!-- Display Mode -->
        <template v-if="editingTask !== task.name">
          <div class="flex items-start justify-between">
            <div>
              <h3 class="text-lg font-semibold text-gray-800">{{ task.name }}</h3>
              <p class="text-sm text-gray-400 mt-1 font-mono">计划表：{{ task.cron }}</p>
            </div>
            <div class="flex items-center gap-2">
              <button
                @click="startEdit(task.name, task.cron, task.max_retries)"
                class="px-3 py-2 text-sm text-gray-500 bg-gray-50 rounded-lg hover:bg-gray-100 hover:text-gray-700 transition-colors cursor-pointer"
                title="编辑任务"
              >✏️ 编辑</button>
              <button
                @click="handleRunTask(task.name)"
                :disabled="runningTask === task.name"
                class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
              >
                {{ runningTask === task.name ? '运行中...' : '▶ 立即运行' }}
              </button>
            </div>
          </div>

          <div class="mt-4 flex flex-wrap gap-4 text-sm">
            <div>
              <span class="text-gray-400">数据源：</span>
              <span v-for="s in task.sources" :key="s"
                class="inline-block bg-blue-50 text-blue-700 px-2 py-0.5 rounded text-xs mr-1"
              >{{ s }}</span>
            </div>
            <div>
              <span class="text-gray-400">推送渠道：</span>
              <span v-for="c in task.channels" :key="c"
                class="inline-block bg-green-50 text-green-700 px-2 py-0.5 rounded text-xs mr-1"
              >{{ c }}</span>
            </div>
            <div class="text-gray-400">
              重试次数：{{ task.max_retries }} | 自动启动：{{ task.run_on_start ? '是' : '否' }}
            </div>
          </div>
        </template>

        <!-- Edit Mode -->
        <template v-else>
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <h3 class="text-lg font-semibold text-blue-600">编辑任务</h3>
              <div class="flex items-center gap-2">
                <button @click="saveEdit(task.name)"
                  class="px-4 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors cursor-pointer">
                  💾 保存
                </button>
                <button @click="cancelEdit"
                  class="px-4 py-2 text-sm bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 transition-colors cursor-pointer">
                  取消
                </button>
              </div>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label class="block text-sm text-gray-500 mb-1">任务名称</label>
                <input
                  v-model="editName"
                  type="text"
                  class="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  placeholder="输入任务名称"
                />
              </div>
              <div>
                <label class="block text-sm text-gray-500 mb-1">Cron 表达式</label>
                <input
                  v-model="editCron"
                  type="text"
                  class="w-full px-3 py-2 text-sm font-mono border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  placeholder="0 0 10 * * *"
                />
              </div>
              <div>
                <label class="block text-sm text-gray-500 mb-1">最大重试次数</label>
                <input
                  v-model.number="editRetries"
                  type="number"
                  min="0"
                  max="10"
                  class="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
            </div>

            <div class="flex flex-wrap gap-4 text-sm pt-2 border-t border-gray-100">
              <div>
                <span class="text-gray-400">数据源：</span>
                <span v-for="s in task.sources" :key="s"
                  class="inline-block bg-blue-50 text-blue-700 px-2 py-0.5 rounded text-xs mr-1"
                >{{ s }}</span>
              </div>
              <div>
                <span class="text-gray-400">推送渠道：</span>
                <span v-for="c in task.channels" :key="c"
                  class="inline-block bg-green-50 text-green-700 px-2 py-0.5 rounded text-xs mr-1"
                >{{ c }}</span>
              </div>
              <div class="text-gray-400">
                自动启动：{{ task.run_on_start ? '是' : '否' }}
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
