<template>
  <div class="data-card">
    <div class="data-card__header" v-if="title || $slots.header">
      <slot name="header">
        <span class="data-card__title">{{ title }}</span>
      </slot>
      <div class="data-card__toolbar" v-if="$slots.toolbar">
        <slot name="toolbar" />
      </div>
    </div>
    <div class="data-card__body">
      <slot />
    </div>
  </div>
</template>

<script setup>
defineProps({
  title: { type: String, default: '' },
})
</script>

<style scoped>
.data-card {
  background-color: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-base);
  overflow: hidden;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.data-card:hover {
  border-color: var(--border-accent);
  box-shadow: var(--shadow-accent), var(--shadow-sm);
}

.data-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  background-color: var(--bg-elevated);
}

.data-card__title {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-secondary);
}

.data-card__toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.data-card__body {
  padding: var(--space-4);
}
</style>
