import { defineStore } from 'pinia'
import * as api from '../api/gb28181'

export const useGb28181RefStore = defineStore('gb28181Ref', {
  state: () => ({
    deviceTypes: [],
    industryCodes: [],
    networkCodes: [],
    loading: false,
    loaded: false,
  }),

  getters: {
    networkCodeOptions: (state) => {
      return state.networkCodes.map(n => ({
        label: `${n.name} (${n.code})`,
        value: n.code,
      }))
    },
    
    industryCodeOptions: (state) => {
      return state.industryCodes.map(i => ({
        label: `${i.name} (${i.code})`,
        value: i.code,
      }))
    },
    
    deviceTypeOptions: (state) => {
      const categories = {
        'frontend_device': { label: '前端主设备', order: 1 },
        'frontend_peripheral': { label: '前端外围设备', order: 2 },
        'platform': { label: '平台设备', order: 3 },
        'center_user': { label: '中心用户', order: 4 },
        'terminal_user': { label: '终端用户', order: 5 },
        'platform_external': { label: '平台外接服务器', order: 6 },
      }
      
      // Group by category
      const grouped = {}
      for (const dt of state.deviceTypes) {
        const cat = dt.category || 'other'
        if (!grouped[cat]) grouped[cat] = []
        grouped[cat].push({
          label: `${dt.name} (${dt.code})`,
          value: dt.code,
        })
      }
      
      // Sort and format - interleave categories with their items
      const result = []
      const sortedCategories = Object.entries(categories).sort((a, b) => a[1].order - b[1].order)
      for (const [catKey, catInfo] of sortedCategories) {
        if (grouped[catKey]) {
          result.push({ type: 'header', label: catInfo.label })
          for (const opt of grouped[catKey]) {
            result.push({ type: 'item', ...opt })
          }
        }
      }
      
      return result
    },
    
    getNetworkCode: (state) => (deviceTypeCode) => {
      // Business Group (215) and Virtual Organization (216) use network code '7'
      if (deviceTypeCode === '215' || deviceTypeCode === '216') {
        return '7'
      }
      // Default is '0' for regular devices
      return '0'
    },
  },

  actions: {
    async fetchRefData() {
      if (this.loaded) return
      this.loading = true
      try {
        const data = await api.getGb28181RefData()
        this.deviceTypes = data.device_types || []
        this.industryCodes = data.industry_codes || []
        this.networkCodes = data.network_codes || []
        this.loaded = true
      } finally {
        this.loading = false
      }
    },
  },
})
