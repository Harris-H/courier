<script setup lang="ts">
import { useToastStore } from '../stores/toast'

const toast = useToastStore()

const typeStyles = {
  success: 'bg-green-500',
  error: 'bg-red-500',
  info: 'bg-blue-500',
}

const typeIcons = {
  success: '✅',
  error: '❌',
  info: 'ℹ️',
}
</script>

<template>
  <Teleport to="body">
    <div class="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      <TransitionGroup name="toast">
        <div
          v-for="t in toast.toasts"
          :key="t.id"
          :class="[typeStyles[t.type], 'pointer-events-auto flex items-center gap-2 px-4 py-3 rounded-lg shadow-lg text-white text-sm min-w-[280px] max-w-[420px]']"
        >
          <span>{{ typeIcons[t.type] }}</span>
          <span class="flex-1">{{ t.message }}</span>
          <button
            @click="toast.dismiss(t.id)"
            class="ml-2 text-white/70 hover:text-white transition-colors cursor-pointer text-lg leading-none"
          >×</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active {
  transition: all 0.3s ease-out;
}
.toast-leave-active {
  transition: all 0.25s ease-in;
}
.toast-enter-from {
  transform: translateX(100%);
  opacity: 0;
}
.toast-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>
