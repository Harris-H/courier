<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { updateFeishuConfig, updateLlmConfig } from '../api'

const store = useAppStore()
const toast = useToastStore()

const feishuEnabled = ref(false)
const feishuWebhookUrl = ref('')
const feishuSaving = ref(false)
const feishuEditing = ref(false)
const webhookVisible = ref(false)

const llmModel = ref('')
const llmSaving = ref(false)

const availableModels = [
  { value: 'doubao-seed-2-0-lite-260215', label: 'Doubao Seed 2.0 Lite', provider: '火山方舟' },
  { value: 'glm-4-7-251222', label: 'GLM-4.7B', provider: '智谱 AI' },
  { value: 'deepseek-v3-2-251201', label: 'DeepSeek V3.2', provider: 'DeepSeek' },
  { value: 'kimi-k2-thinking-251104', label: 'Kimi K2 Thinking', provider: 'Moonshot AI' },
]

onMounted(async () => {
  await store.fetchConfig()
  if (store.config) {
    feishuEnabled.value = store.config.channels.feishu
    feishuWebhookUrl.value = store.config.feishu_webhook_url || ''
    llmModel.value = store.config.llm_model
  }
})

watch(() => store.config, (cfg) => {
  if (cfg) {
    feishuEnabled.value = cfg.channels.feishu
    feishuWebhookUrl.value = cfg.feishu_webhook_url || ''
    llmModel.value = cfg.llm_model
  }
})

async function saveLlmModel() {
  llmSaving.value = true
  try {
    const res = await updateLlmConfig({ model: llmModel.value })
    if (res.data.success) {
      toast.success(res.data.message)
      await store.fetchConfig()
    } else {
      toast.error(res.data.message)
    }
  } catch (e: any) {
    toast.error(`保存失败：${e.message}`)
  } finally {
    llmSaving.value = false
  }
}

async function saveFeishuConfig() {
  feishuSaving.value = true
  try {
    const res = await updateFeishuConfig({
      enabled: feishuEnabled.value,
      webhook_url: feishuWebhookUrl.value,
    })
    if (res.data.success) {
      toast.success(res.data.message)
      feishuEditing.value = false
    } else {
      toast.error(res.data.message)
    }
  } catch (e: any) {
    toast.error(`保存失败：${e.message}`)
  } finally {
    feishuSaving.value = false
  }
}

function cancelFeishuEdit() {
  if (store.config) {
    feishuEnabled.value = store.config.channels.feishu
    feishuWebhookUrl.value = store.config.feishu_webhook_url || ''
  }
  feishuEditing.value = false
}
</script>

<template>
  <div>
    <h2 class="text-2xl font-bold text-gray-800 mb-6">配置</h2>

    <div v-if="!store.config" class="text-gray-400 text-center py-12">加载中...</div>

    <template v-else>
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- General -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">常规</h3>
          <div class="space-y-3 text-sm">
            <div class="flex justify-between">
              <span class="text-gray-500">日志级别</span>
              <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded">{{ store.config.log_level }}</span>
            </div>
          </div>
        </div>

        <!-- LLM -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">大语言模型</h3>
          <div class="space-y-3 text-sm">
            <div>
              <label class="block text-gray-500 mb-1">模型</label>
              <div class="flex items-center gap-2">
                <select
                  v-model="llmModel"
                  class="flex-1 px-3 py-2 text-sm border border-gray-300 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 cursor-pointer"
                >
                  <option v-for="m in availableModels" :key="m.value" :value="m.value">
                    {{ m.label }}（{{ m.provider }}）
                  </option>
                </select>
                <button
                  @click="saveLlmModel"
                  :disabled="llmSaving || llmModel === store.config?.llm_model"
                  class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer whitespace-nowrap"
                >
                  {{ llmSaving ? '保存中...' : '保存' }}
                </button>
              </div>
              <p v-if="llmModel !== store.config?.llm_model" class="text-xs text-amber-500 mt-1">
                ⚠️ 模型已更改，点击保存生效（下次任务执行时将使用新模型）
              </p>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-500">API 地址</span>
              <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded text-xs">{{ store.config.llm_api_base }}</span>
            </div>
          </div>
        </div>

        <!-- Sources -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">📰 数据源</h3>
          <div class="space-y-3 text-sm">
            <div class="flex items-center justify-between" v-for="(enabled, name) in store.config.sources" :key="name">
              <span class="text-gray-700 capitalize">{{ name }}</span>
              <span :class="enabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-400'"
                class="px-2 py-0.5 rounded text-xs font-medium"
              >
                {{ enabled ? '已启用' : '已禁用' }}
              </span>
            </div>
          </div>
        </div>

        <!-- Channels -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">📤 推送渠道</h3>
          <div class="space-y-3 text-sm">
            <div class="flex items-center justify-between" v-for="(enabled, name) in store.config.channels" :key="name">
              <span class="text-gray-700 capitalize">{{ name }}</span>
              <span :class="enabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-400'"
                class="px-2 py-0.5 rounded text-xs font-medium"
              >
                {{ enabled ? '已启用' : '已禁用' }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Feishu Config Editor -->
      <div class="mt-6 bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-sm font-semibold text-gray-500 uppercase">🔧 飞书渠道配置</h3>
          <button
            v-if="!feishuEditing"
            @click="feishuEditing = true"
            class="px-3 py-1 text-xs bg-blue-50 text-blue-600 rounded-lg hover:bg-blue-100 transition-colors cursor-pointer"
          >
            ✏️ 编辑
          </button>
        </div>

        <!-- Feedback removed: using toast now -->

        <!-- Read-only view -->
        <div v-if="!feishuEditing" class="space-y-3 text-sm">
          <div class="flex justify-between">
            <span class="text-gray-500">状态</span>
            <span :class="feishuEnabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-400'"
              class="px-2 py-0.5 rounded text-xs font-medium"
            >
              {{ feishuEnabled ? '已启用' : '已禁用' }}
            </span>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-gray-500">Webhook 地址</span>
            <div class="flex items-center gap-2">
              <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded text-xs max-w-xs truncate">
                {{ feishuWebhookUrl ? (webhookVisible ? feishuWebhookUrl : '••••••••••••••••') : '未配置' }}
              </span>
              <button
                v-if="feishuWebhookUrl"
                @click="webhookVisible = !webhookVisible"
                class="text-gray-400 hover:text-gray-600 text-sm cursor-pointer"
                :title="webhookVisible ? '隐藏' : '显示'"
              >
                {{ webhookVisible ? '🙈' : '👁️' }}
              </button>
            </div>
          </div>
        </div>

        <!-- Edit form -->
        <div v-else class="space-y-4">
          <div class="flex items-center gap-3">
            <label class="text-sm text-gray-500 w-24">启用状态</label>
            <label class="relative inline-flex items-center cursor-pointer">
              <input type="checkbox" v-model="feishuEnabled" class="sr-only peer">
              <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600"></div>
              <span class="ms-2 text-sm text-gray-600">{{ feishuEnabled ? '已启用' : '已禁用' }}</span>
            </label>
          </div>
          <div>
            <label class="block text-sm text-gray-500 mb-1">Webhook 地址</label>
            <div class="relative">
              <input
                v-model="feishuWebhookUrl"
                :type="webhookVisible ? 'text' : 'password'"
                placeholder="https://open.feishu.cn/open-apis/bot/v2/hook/xxxxx"
                class="w-full px-3 py-2 pr-10 border border-gray-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <button
                type="button"
                @click="webhookVisible = !webhookVisible"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 cursor-pointer"
              >
                {{ webhookVisible ? '🙈' : '👁️' }}
              </button>
            </div>
            <p class="text-xs text-gray-400 mt-1">在飞书群组中添加自定义机器人后获取 Webhook 地址</p>
          </div>
          <div class="flex gap-2 pt-2">
            <button
              @click="saveFeishuConfig"
              :disabled="feishuSaving"
              class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 transition-colors cursor-pointer"
            >
              {{ feishuSaving ? '保存中...' : '💾 保存' }}
            </button>
            <button
              @click="cancelFeishuEdit"
              class="px-4 py-2 bg-gray-100 text-gray-600 text-sm rounded-lg hover:bg-gray-200 transition-colors cursor-pointer"
            >
              取消
            </button>
          </div>
        </div>
      </div>

      <div class="mt-6 p-4 bg-amber-50 border border-amber-200 rounded-xl text-sm text-amber-700">
        💡 配置修改请编辑 <code class="bg-amber-100 px-1 rounded">config.toml</code> 文件后重启服务。
      </div>
    </template>
  </div>
</template>
