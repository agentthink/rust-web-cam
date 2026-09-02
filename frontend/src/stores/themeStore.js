import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  const saved = localStorage.getItem('rustcam_theme')
  const isDark = ref(saved ? saved === 'dark' : true)

  function toggle() {
    isDark.value = !isDark.value
    apply()
  }

  function apply() {
    const theme = isDark.value ? 'dark' : 'light'
    document.documentElement.classList.toggle('dark', isDark.value)
    localStorage.setItem('rustcam_theme', theme)
  }

  apply()

  watch(isDark, apply)

  return { isDark, toggle, apply }
})
