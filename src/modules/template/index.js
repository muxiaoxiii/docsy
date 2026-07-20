export default {
  id: 'template',
  name: '文书模板',
  icon: 'Document',
  description: '用 Word 标黄制作可填写模板',
  category: 'document',
  order: 60,
  defaultVisible: true,

  routes: [
    {
      path: '/template',
      name: 'template',
      component: () => import('./views/TemplateView.vue'),
      meta: { title: '文书模板', moduleId: 'template' },
    },
  ],

  menuItems: [{ label: '文书模板', route: 'template', icon: 'Document' }],

  homeCards: [
    {
      title: '文书模板',
      description: 'Word 标黄字段，批量生成文书',
      route: 'template',
      icon: 'Document',
    },
  ],

  settings: null,
}
