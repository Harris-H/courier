<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useToastStore } from '../stores/toast'
import { updateFeishuConfig, updateEmailConfig, updateLlmConfig } from '../api'

const store = useAppStore()
const toast = useToastStore()

const feishuEnabled = ref(false)
const feishuWebhookUrl = ref('')
const feishuSaving = ref(false)
const feishuEditing = ref(false)
const webhookVisible = ref(false)

const llmModel = ref('')
const llmMaxTokens = ref(4000)
const llmSaving = ref(false)

const emailEnabled = ref(false)
const emailSmtpHost = ref('')
const emailSmtpPort = ref(465)
const emailSmtpUsername = ref('')
const emailSmtpPassword = ref('')
const emailFrom = ref('')
const emailTo = ref('')
const emailSaving = ref(false)
const emailEditing = ref(false)
const emailPasswordVisible = ref(false)

const availableModels = [
  { value: 'ep-20260404123347-5lprz', label: 'Doubao Seed 2.0 Lite', provider: '火山方舟' },
  { value: 'ep-20260404125954-zfgwz', label: 'GLM-4.7B', provider: '智谱 AI' },
  { value: 'ep-20260404125909-wzgdz', label: 'DeepSeek V3.2', provider: 'DeepSeek' },
  { value: 'kimi-k2-thinking-251104', label: 'Kimi K2 Thinking', provider: 'Moonshot AI' },
]

onMounted(async () => {
  await store.fetchConfig()
  if (store.config) {
    feishuEnabled.value = store.config.channels.feishu
    feishuWebhookUrl.value = store.config.feishu_webhook_url || ''
    llmModel.value = store.config.llm_model
    llmMaxTokens.value = store.config.llm_max_tokens || 4000
    loadEmailConfig(store.config)
  }
})

watch(() => store.config, (cfg) => {
  if (cfg) {
    feishuEnabled.value = cfg.channels.feishu
    feishuWebhookUrl.value = cfg.feishu_webhook_url || ''
    llmModel.value = cfg.llm_model
    llmMaxTokens.value = cfg.llm_max_tokens || 4000
    loadEmailConfig(cfg)
  }
})

async function saveLlmModel() {
  llmSaving.value = true
  try {
    const res = await updateLlmConfig({ model: llmModel.value, max_tokens: llmMaxTokens.value })
    if (res.data.success) {
      toast.success(res.data.message)
      await store.fetchConfig()
    } else {
      toast.error(res.data.message)
    }
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '未知错误'
    toast.error(`保存失败：${msg}`)
  } finally {
    llmSaving.value = false
  }
}

async function saveFeishuConfig() {
  // Validate webhook URL when enabling
  if (feishuEnabled.value && !feishuWebhookUrl.value.trim()) {
    toast.error('启用飞书时必须提供 Webhook 地址')
    return
  }
  if (feishuWebhookUrl.value.trim()) {
    try {
      new URL(feishuWebhookUrl.value.trim())
    } catch {
      toast.error('Webhook 地址格式无效，请输入完整 URL')
      return
    }
  }

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
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '未知错误'
    toast.error(`保存失败：${msg}`)
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

function loadEmailConfig(cfg: typeof store.config) {
  if (!cfg) return
  const ec = cfg.email_config
  emailEnabled.value = ec?.enabled ?? false
  emailSmtpHost.value = ec?.smtp_host ?? ''
  emailSmtpPort.value = ec?.smtp_port ?? 465
  emailSmtpUsername.value = ec?.smtp_username ?? ''
  emailFrom.value = ec?.from ?? ''
  emailTo.value = ec?.to?.join(', ') ?? ''
  emailSmtpPassword.value = ''
}

async function saveEmailConfig() {
  if (emailEnabled.value) {
    if (!emailSmtpHost.value.trim()) {
      toast.error('启用邮件时必须提供 SMTP 服务器地址')
      return
    }
    if (!emailFrom.value.trim()) {
      toast.error('启用邮件时必须提供发件人地址')
      return
    }
    if (!emailTo.value.trim()) {
      toast.error('启用邮件时必须提供至少一个收件人')
      return
    }
  }

  emailSaving.value = true
  try {
    const recipients = emailTo.value
      .split(/[,;，；\n]+/)
      .map(s => s.trim())
      .filter(Boolean)
    const res = await updateEmailConfig({
      enabled: emailEnabled.value,
      smtp_host: emailSmtpHost.value.trim(),
      smtp_port: emailSmtpPort.value,
      smtp_username: emailSmtpUsername.value.trim(),
      smtp_password: emailSmtpPassword.value,
      from: emailFrom.value.trim(),
      to: recipients,
    })
    if (res.data.success) {
      toast.success(res.data.message)
      emailEditing.value = false
      await store.fetchConfig()
    } else {
      toast.error(res.data.message)
    }
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '未知错误'
    toast.error(`保存失败：${msg}`)
  } finally {
    emailSaving.value = false
  }
}

function cancelEmailEdit() {
  if (store.config) {
    loadEmailConfig(store.config)
  }
  emailEditing.value = false
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
                  :disabled="llmSaving || (llmModel === store.config?.llm_model && llmMaxTokens === store.config?.llm_max_tokens)"
                  class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer whitespace-nowrap"
                >
                  {{ llmSaving ? '保存中...' : '保存' }}
                </button>
              </div>
              <p v-if="llmModel !== store.config?.llm_model || llmMaxTokens !== store.config?.llm_max_tokens" class="text-xs text-amber-500 mt-1">
                ⚠️ 配置已更改，点击保存生效（下次任务执行时将使用新配置）
              </p>
            </div>
            <div>
              <label class="block text-gray-500 mb-1">最大 Tokens</label>
              <input
                v-model.number="llmMaxTokens"
                type="number"
                min="1000"
                max="65535"
                step="1000"
                class="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                placeholder="4000"
              />
              <p class="text-xs text-gray-400 mt-1">控制 LLM 输出长度，建议 4000-8000，过小会导致内容截断</p>
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

      <!-- Email Config Editor -->
      <div class="mt-6 bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-sm font-semibold text-gray-500 uppercase">📧 邮件渠道配置</h3>
          <button
            v-if="!emailEditing"
            @click="emailEditing = true"
            class="px-3 py-1 text-xs bg-blue-50 text-blue-600 rounded-lg hover:bg-blue-100 transition-colors cursor-pointer"
          >
            ✏️ 编辑
          </button>
        </div>

        <!-- Read-only view -->
        <div v-if="!emailEditing" class="space-y-3 text-sm">
          <div class="flex justify-between">
            <span class="text-gray-500">状态</span>
            <span :class="emailEnabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-400'"
              class="px-2 py-0.5 rounded text-xs font-medium"
            >
              {{ emailEnabled ? '已启用' : '已禁用' }}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">SMTP 服务器</span>
            <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded text-xs">
              {{ emailSmtpHost || '未配置' }}{{ emailSmtpHost ? `:${emailSmtpPort}` : '' }}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">发件人</span>
            <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded text-xs">
              {{ emailFrom || '未配置' }}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">收件人</span>
            <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded text-xs max-w-xs truncate">
              {{ emailTo || '未配置' }}
            </span>
          </div>
        </div>

        <!-- Edit form -->
        <div v-else class="space-y-4">
          <div class="flex items-center gap-3">
            <label class="text-sm text-gray-500 w-24">启用状态</label>
            <label class="relative inline-flex items-center cursor-pointer">
              <input type="checkbox" v-model="emailEnabled" class="sr-only peer">
              <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600"></div>
              <span class="ms-2 text-sm text-gray-600">{{ emailEnabled ? '已启用' : '已禁用' }}</span>
            </label>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label class="block text-sm text-gray-500 mb-1">SMTP 服务器</label>
              <input
                v-model="emailSmtpHost"
                type="text"
                placeholder="smtp.gmail.com"
                class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
            <div>
              <label class="block text-sm text-gray-500 mb-1">SMTP 端口</label>
              <input
                v-model.number="emailSmtpPort"
                type="number"
                placeholder="465"
                class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label class="block text-sm text-gray-500 mb-1">SMTP 用户名</label>
              <input
                v-model="emailSmtpUsername"
                type="text"
                placeholder="your-email@gmail.com"
                class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
            <div>
              <label class="block text-sm text-gray-500 mb-1">SMTP 密码 / 授权码</label>
              <div class="relative">
                <input
                  v-model="emailSmtpPassword"
                  :type="emailPasswordVisible ? 'text' : 'password'"
                  :placeholder="store.config?.email_config?.has_password ? '已设置，留空则不修改' : '输入 SMTP 密码或授权码'"
                  class="w-full px-3 py-2 pr-10 border border-gray-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <button
                  type="button"
                  @click="emailPasswordVisible = !emailPasswordVisible"
                  class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 cursor-pointer"
                >
                  {{ emailPasswordVisible ? '🙈' : '👁️' }}
                </button>
              </div>
            </div>
          </div>
          <div>
            <label class="block text-sm text-gray-500 mb-1">发件人</label>
            <input
              v-model="emailFrom"
              type="text"
              placeholder="Courier Bot <your-email@gmail.com>"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          <div>
            <label class="block text-sm text-gray-500 mb-1">收件人（多个用逗号分隔）</label>
            <input
              v-model="emailTo"
              type="text"
              placeholder="recipient1@example.com, recipient2@example.com"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          <div class="flex gap-2 pt-2">
            <button
              @click="saveEmailConfig"
              :disabled="emailSaving"
              class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 transition-colors cursor-pointer"
            >
              {{ emailSaving ? '保存中...' : '💾 保存' }}
            </button>
            <button
              @click="cancelEmailEdit"
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
