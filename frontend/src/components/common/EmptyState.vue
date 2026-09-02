<template>
  <div class="empty-state">
    <div class="empty-icon">
      <slot name="icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M12 2a10 10 0 100 20 10 10 0 000-20z"/>
        </svg>
      </slot>
    </div>
    <div class="empty-title">{{ title }}</div>
    <div class="empty-desc" v-if="description">{{ description }}</div>
    <div class="empty-action" v-if="$slots.action">
      <slot name="action" />
    </div>
  </div>
</template>

<script setup>
defineProps({
  title: { type: String, default: '暂无数据' },
  description: { type: String, default: '' },
})
</script>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-12) var(--space-6);
  gap: var(--space-3);
  width: 100%;
}

.empty-icon {
  color: var(--text-muted);
  opacity: 0.4;
}

.empty-title {
  font-size: var(--text-md);
  font-weight: var(--weight-medium);
  color: var(--text-secondary);
}

.empty-desc {
  font-size: var(--text-sm);
  color: var(--text-muted);
  text-align: center;
}

.empty-action {
  margin-top: var(--space-3);
}
</style>
