<template>
  <span class="status-dot" :class="status"></span>
</template>

<script setup>
defineProps({
  status: {
    type: String,
    default: 'offline',
  }
})
</script>

<style scoped>
.status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.online    { background-color: var(--color-success); animation: pulse-online 2s ease-in-out infinite; }
.status-dot.offline { background-color: var(--status-offline); }
.status-dot.warning  { background-color: var(--color-warning); }
.status-dot.error    { background-color: var(--color-danger); }
.status-dot.maintaining { background-color: var(--color-warning); animation: pulse-online 1.5s ease-in-out infinite; }
.status-dot.recording { background-color: var(--color-danger); animation: pulse-online 1.5s ease-in-out infinite; }
</style>
