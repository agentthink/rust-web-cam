import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import * as ElementPlusIcons from '@element-plus/icons-vue'
import 'element-plus/dist/index.css'
import router from './router'
import App from './App.vue'
import './assets/main.scss'
import VChart from 'vue-echarts'
import * as echarts from 'echarts/core'
import { SVGRenderer } from 'echarts/renderers'
import { LineChart, BarChart, PieChart, GaugeChart, RadarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent, TitleComponent } from 'echarts/components'

echarts.use([SVGRenderer, LineChart, BarChart, PieChart, GaugeChart, RadarChart, GridComponent, TooltipComponent, LegendComponent, TitleComponent])


const app = createApp(App)
const pinia = createPinia()
app.component('v-chart', VChart)

app.use(pinia)
app.use(router)
app.use(ElementPlus, { size: 'small' })

if (window.ClientApp) {
  const wsUrl = `ws://${window.location.hostname}:8080`
  window.clientApp = new window.ClientApp(wsUrl)
  console.log('[main] ClientApp initialized, wsUrl:', wsUrl)
} else {
  console.warn('[main] ClientApp not available (rtsp-player not loaded)')
}
for (const [key, component] of Object.entries(ElementPlusIcons)) {
  app.component(key, component)
}

app.mount('#app')

import { useThemeStore } from './stores/themeStore'
useThemeStore()


