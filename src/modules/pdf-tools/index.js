export default {
  id: 'pdf-tools',
  name: '文档工具',
  icon: 'Files',
  description: 'PDF 解锁、合并、拆分',
  category: 'pdf',
  order: 30,
  defaultVisible: true,

  routes: [
    {
      path: '/pdf/:tab?',
      name: 'pdf-tools',
      component: () => import('./views/PdfToolsView.vue'),
      meta: { title: '文档工具', moduleId: 'pdf-tools' },
    },
  ],

  menuItems: [{ label: '文档工具', route: 'pdf-tools', icon: 'Files' }],

  homeCards: [
    {
      title: '文档工具',
      description: '解锁、合并、拆分、Markdown 互转',
      route: 'pdf-tools',
      icon: 'Files',
    },
  ],

  settings: null,
}
