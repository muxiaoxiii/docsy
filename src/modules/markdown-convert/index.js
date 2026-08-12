export default {
  id: 'markdown-convert',
  name: 'MD↔Word',
  icon: 'Document',
  description: 'Markdown 与 Word 文档双向转换',
  category: 'document',
  order: 35,
  defaultVisible: true,

  routes: [
    {
      path: '/markdown',
      name: 'markdown-convert',
      component: () => import('./views/MarkdownConvertView.vue'),
      meta: { title: 'MD↔Word', moduleId: 'markdown-convert' },
    },
  ],

  menuItems: [{ label: 'MD↔Word', route: 'markdown-convert', icon: 'Document' }],

  homeCards: [
    {
      title: 'MD↔Word',
      description: 'Markdown 与 Word 双向转换，也可粘贴 Markdown 直接生成文档',
      route: 'markdown-convert',
      icon: 'Document',
    },
  ],

  settings: null,
}
