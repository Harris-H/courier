<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '../stores/app'

const store = useAppStore()

onMounted(() => store.fetchConfig())
</script>

<template>
  <div>
    <h2 class="text-2xl font-bold text-gray-800 mb-6">Configuration</h2>

    <div v-if="!store.config" class="text-gray-400 text-center py-12">Loading...</div>

    <template v-else>
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- General -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">General</h3>
          <div class="space-y-3 text-sm">
            <div class="flex justify-between">
              <span class="text-gray-500">Log Level</span>
              <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded">{{ store.config.log_level }}</span>
            </div>
          </div>
        </div>

        <!-- LLM -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">LLM</h3>
          <div class="space-y-3 text-sm">
            <div class="flex justify-between">
              <span class="text-gray-500">Model</span>
              <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded">{{ store.config.llm_model }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-500">API Base</span>
              <span class="font-mono text-gray-700 bg-gray-100 px-2 py-0.5 rounded text-xs">{{ store.config.llm_api_base }}</span>
            </div>
          </div>
        </div>

        <!-- Sources -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">📰 Sources</h3>
          <div class="space-y-3 text-sm">
            <div class="flex items-center justify-between" v-for="(enabled, name) in store.config.sources" :key="name">
              <span class="text-gray-700 capitalize">{{ name }}</span>
              <span :class="enabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-400'"
                class="px-2 py-0.5 rounded text-xs font-medium"
              >
                {{ enabled ? 'Enabled' : 'Disabled' }}
              </span>
            </div>
          </div>
        </div>

        <!-- Channels -->
        <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm">
          <h3 class="text-sm font-semibold text-gray-500 uppercase mb-4">📤 Channels</h3>
          <div class="space-y-3 text-sm">
            <div class="flex items-center justify-between" v-for="(enabled, name) in store.config.channels" :key="name">
              <span class="text-gray-700 capitalize">{{ name }}</span>
              <span :class="enabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-400'"
                class="px-2 py-0.5 rounded text-xs font-medium"
              >
                {{ enabled ? 'Enabled' : 'Disabled' }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <div class="mt-6 p-4 bg-amber-50 border border-amber-200 rounded-xl text-sm text-amber-700">
        💡 配置修改请编辑 <code class="bg-amber-100 px-1 rounded">config.toml</code> 文件后重启服务。
      </div>
    </template>
  </div>
</template>
