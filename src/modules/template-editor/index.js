export default {
  id: 'template-editor',
  name: '模板编辑',
  icon: 'Edit',
  description: '制作和编辑 .docsytpl 模板',
  category: 'document',
  defaultVisible: true,

  routes: [
    {
      path: '/template-editor/:templateId?',
      name: 'template-editor',
      component: () => import('./views/TemplateEditorView.vue'),
      meta: { title: '模板编辑', moduleId: 'template-editor' },
    },
  ],

  menuItems: [
    { label: '模板编辑', route: 'template-editor', icon: 'Edit' },
  ],

  homeCards: [
    {
      title: '制作模板',
      description: '从 Word 文档创建可复用模板',
      route: 'template-editor',
      icon: 'Edit',
    },
  ],

  settings: null,
}
