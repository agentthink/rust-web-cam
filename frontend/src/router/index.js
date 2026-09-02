import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/authStore'

const routes = [
  {
    path: '/',
    component: () => import('../components/layout/AppLayout.vue'),
    meta: { requiresAuth: true },
    children: [
      {
        path: '',
        name: 'Dashboard',
        component: () => import('../views/Dashboard.vue'),
      },
      {
        path: 'devices',
        name: 'Devices',
        component: () => import('../views/Devices.vue'),
      },
      {
        path: 'devices/:deviceTag',
        name: 'DeviceDetail',
        component: () => import('../views/DeviceDetail.vue'),
      },
      {
        path: 'channels',
        name: 'Channels',
        component: () => import('../views/Channels.vue'),
      },
      {
        path: 'channels/:deviceTag/:channelTag',
        name: 'ChannelDetail',
        component: () => import('../views/ChannelDetail.vue'),
      },
      {
        path: 'streams',
        name: 'Streams',
        component: () => import('../views/Streams.vue'),
      },
      {
        path: 'streams/:id',
        name: 'StreamDetail',
        component: () => import('../views/StreamDetail.vue'),
      },
      {
        path: 'sessions',
        name: 'Sessions',
        component: () => import('../views/Sessions.vue'),
      },
      {
        path: 'public',
        name: 'Public',
        component: () => import('../views/PublicCameras.vue'),
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('../views/Settings.vue'),
      },
      {
        path: 'servers',
        name: 'Servers',
        component: () => import('../views/MediaServers.vue'),
      },
      {
        path: 'recordings',
        name: 'Recordings',
        component: () => import('../views/Recordings.vue'),
      },
      {
        path: 'recordings/files',
        name: 'RecordingFiles',
        component: () => import('../views/RecordingFiles.vue'),
      },
      {
        path: 'users',
        name: 'UserManagement',
        component: () => import('../views/UserManagement.vue'),
      },
      {
        path: 'alarms',
        name: 'Alarms',
        component: () => import('../views/Alarms.vue'),
      },
      {
        path: 'video-wall',
        name: 'PlayerWall',
        component: () => import('../views/PlayerWallWrapper.vue'),
      },
      {
        path: 'video-wall/designer',
        name: 'PlayerWallDesigner',
        component: () => import('../views/PlayerWallDesigner.vue'),
      },
    ],
  },
  {
    path: '/login',
    name: 'Login',
    component: () => import('../views/Login.vue'),
    meta: { public: true },
  },
]

const router = createRouter({
  history: createWebHistory('/'),
  routes,
})

router.beforeEach((to, from, next) => {
  const token = localStorage.getItem('rustcam_access_token')
  if (to.meta.public) {
    if (token && to.name === 'Login') {
      return next({ name: 'Dashboard' })
    }
    next()
  } else if (!token) {
    next({ name: 'Login', query: { redirect: window.location.pathname } })
  } else {
    next()
  }
})

export default router
