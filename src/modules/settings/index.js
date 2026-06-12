export default {
  id: 'settings',
  name: '设置',
  icon: 'Setting',
  description: '应用设置',
  category: 'system',
  defaultVisible: true,

  routes: [
    {
      path: '/settings',
      name: 'settings',
      component: () => import('./views/SettingsView.vue'),
      meta: { title: '设置', moduleId: 'settings' },
    },
  ],

  menuItems: [
    { label: '设置', route: 'settings', icon: 'Setting' },
  ],

  homeCards: [],
  settings: null,
}
