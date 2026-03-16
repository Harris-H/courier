<script setup lang="ts">
import { RouterLink, RouterView, useRoute } from 'vue-router'
import ToastContainer from './components/ToastContainer.vue'

const route = useRoute()

const navItems = [
  { path: '/', label: '📊 仪表盘', name: 'dashboard' },
  { path: '/tasks', label: '⏰ 定时任务', name: 'tasks' },
  { path: '/history', label: '📋 执行历史', name: 'history' },
  { path: '/config', label: '⚙️ 配置', name: 'config' },
]
</script>

<template>
  <div class="min-h-screen bg-gray-50 flex">
    <!-- Sidebar -->
    <aside class="w-60 bg-white border-r border-gray-200 flex flex-col">
      <div class="p-5 border-b border-gray-200">
        <h1 class="text-xl font-bold text-gray-800 flex items-center gap-2">
          📬 Courier
        </h1>
        <p class="text-xs text-gray-400 mt-1">新闻摘要机器人</p>
      </div>
      <nav class="flex-1 p-3 space-y-1">
        <RouterLink
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          :class="[
            'block px-3 py-2 rounded-lg text-sm transition-colors',
            route.name === item.name
              ? 'bg-blue-50 text-blue-700 font-medium'
              : 'text-gray-600 hover:bg-gray-100'
          ]"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
      <div class="p-4 border-t border-gray-200 text-xs text-gray-400">
        由 Rust + Vue 驱动
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 p-6 overflow-auto">
      <RouterView />
    </main>

    <ToastContainer />
  </div>
</template>
