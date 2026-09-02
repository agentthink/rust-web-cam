<template>
  <div >
    <div>
      <div>
        <div>{{ title }}</div>
        <div>
          <span >{{ value }}</span>
          <span v-if="total !== undefined">/ {{ total }}</span>
        </div>
        <div v-if="online !== undefined">
          <span>{{ online }}</span> 在线
        </div>
      </div>
      <div >
        <svg v-if="icon === 'camera'"  fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
        </svg>
        <svg v-else-if="icon === 'stream'"  fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"></path>
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
        </svg>
        <svg v-else-if="icon === 'eye'"  fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
        </svg>
        <svg v-else-if="icon === 'server'"  fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"></path>
        </svg>
      </div>
    </div>
    <div>
      <div  ></div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  title: String,
  value: [Number, String],
  total: Number,
  online: Number,
  icon: String,
  color: {
    type: String,
    default: 'cyan'
  }
})

const colorClass = computed(() => {
  const classes = {
    cyan: 'border-accent-cyan/20',
    green: 'border-status-online/20',
    red: 'border-accent-red/20',
    blue: 'border-accent-blue/20',
    amber: 'border-status-warning/20',
    purple: 'border-accent-purple/20'
  }
  return classes[props.color] || classes.cyan
})

const valueColorClass = computed(() => {
  const classes = {
    cyan: 'text-accent-cyan',
    green: 'text-status-online',
    red: 'text-accent-red',
    blue: 'text-accent-blue',
    amber: 'text-status-warning',
    purple: 'text-accent-purple'
  }
  return classes[props.color] || classes.cyan
})

const iconBgClass = computed(() => {
  const classes = {
    cyan: 'bg-accent-cyan/10',
    green: 'bg-status-online/10',
    red: 'bg-accent-red/10',
    blue: 'bg-accent-blue/10',
    amber: 'bg-status-warning/10',
    purple: 'bg-accent-purple/10'
  }
  return classes[props.color] || classes.cyan
})

const iconColorClass = computed(() => {
  const classes = {
    cyan: 'text-accent-cyan',
    green: 'text-status-online',
    red: 'text-accent-red',
    blue: 'text-accent-blue',
    amber: 'text-status-warning',
    purple: 'text-accent-purple'
  }
  return classes[props.color] || classes.cyan
})

const progressColorClass = computed(() => {
  const classes = {
    cyan: 'bg-accent-cyan',
    green: 'bg-status-online',
    red: 'bg-accent-red',
    blue: 'bg-accent-blue',
    amber: 'bg-status-warning',
    purple: 'bg-accent-purple'
  }
  return classes[props.color] || classes.cyan
})

const progressWidth = computed(() => {
  if (props.total && props.total > 0) {
    return `${(props.value / props.total) * 100}%`
  }
  return '50%'
})
</script>
