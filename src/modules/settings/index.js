export default {
  id: 'settings',
  name: '设置',
  icon: 'Setting',
  description: '应用设置',
  category: 'system',
  order: 900,
  defaultVisible: true,

  routes: [
    {
      path: '/settings',
      name: 'settings',
      component: () => import('./views/SettingsView.vue'),
      meta: { title: '设置', moduleId: 'settings' },
    },
  ],

  menuItems: [],

  homeCards: [],
  settings: null,
}
