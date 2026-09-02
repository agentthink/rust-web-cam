<template>
  <div class="metric-card" :class="{ 'is-clickable': clickable }">
    <div class="metric-value">{{ displayValue }}</div>
    <div class="metric-label">{{ label }}</div>
    <div class="metric-extra" v-if="$slots.extra">
      <slot name="extra" />
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  value: { type: [Number, String], default: 0 },
  label: { type: String, default: '' },
  clickable: { type: Boolean, default: false },
})

const displayValue = computed(() => {
  if (typeof props.value === 'number') {
    return props.value.toLocaleString('zh-CN')
  }
  return props.value
})
</script>

<style scoped>
.metric-card {
  background-color: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-base);
  padding: var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  transition: border-color 0.15s, box-shadow 0.15s, transform 0.15s;
}

.metric-card.is-clickable {
  cursor: pointer;
}

.metric-card:hover {
  border-color: var(--border-accent);
  box-shadow: var(--shadow-accent);
  transform: translateY(-1px);
}

.metric-value {
  font-family: var(--font-mono);
  font-size: var(--text-2xl);
  font-weight: var(--weight-bold);
  color: var(--color-accent);
  line-height: 1;
  letter-spacing: -1px;
}

.metric-label {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  font-family: var(--font-cn);
}

.metric-extra {
  margin-top: var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
</style>
