import { ElMessage } from 'element-plus'

export function useToast() {
  function success(message, duration = 3000) {
    ElMessage({ message, type: 'success', duration })
  }
  function error(message, duration = 4000) {
    ElMessage({ message, type: 'error', duration })
  }
  function warning(message, duration = 4000) {
    ElMessage({ message, type: 'warning', duration })
  }
  function info(message, duration = 4000) {
    ElMessage({ message, type: 'info', duration })
  }
  function addToast({ message, type = 'info', duration = 4000 }) {
    ElMessage({ message, type, duration })
  }

  return { success, error, warning, info, addToast }
}
