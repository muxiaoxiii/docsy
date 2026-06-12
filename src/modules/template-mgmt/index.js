export default {
  id: 'template-mgmt',
  name: '模板管理',
  icon: 'FolderOpened',
  description: '管理模板、字典、归档',
  category: 'document',
  defaultVisible: true,

  routes: [
    {
      path: '/templates',
      name: 'template-mgmt',
      component: () => import('./views/ManageView.vue'),
      meta: { title: '模板管理', moduleId: 'template-mgmt' },
    },
  ],

  menuItems: [
    { label: '模板管理', route: 'template-mgmt', icon: 'FolderOpened' },
  ],

  homeCards: [],
  settings: null,
}
