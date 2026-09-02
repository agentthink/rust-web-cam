<template>
  <div class="page-container">
    <component :is="currentComponent" />
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useSettingsStore } from '../stores/settingsStore'
import PlayerWallRtsp from './PlayerWallRtsp.vue'
import PlayerWallFlv from './PlayerWallFlv.vue'

const settingsStore = useSettingsStore()

const currentComponent = computed(() => {
  return settingsStore.playerType === 'flv' ? PlayerWallFlv : PlayerWallRtsp
})
</script>

<style scoped>
.page-container {
  min-height: 100%;
}
</style>
