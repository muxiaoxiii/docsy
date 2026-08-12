export default {
  id: 'markdown-convert',
  name: 'MD转换',
  icon: 'Document',
  description: 'Markdown 与 Office 文档双向转换',
  category: 'document',
  order: 35,
  defaultVisible: true,

  routes: [
    {
      path: '/markdown',
      name: 'markdown-convert',
      component: () => import('./views/MarkdownConvertView.vue'),
      meta: { title: 'MD转换', moduleId: 'markdown-convert' },
    },
  ],

  menuItems: [{ label: 'MD转换', route: 'markdown-convert', icon: 'Document' }],

  homeCards: [
    {
      title: 'MD转换',
      description: 'Markdown 与 Word、Excel、PowerPoint 等 Office 文档互转',
      route: 'markdown-convert',
      icon: 'Document',
    },
  ],

  settings: null,
}
