export default {
  id: 'pdf-tools',
  name: 'PDF 工具',
  icon: 'Files',
  description: 'PDF 解锁、合并、拆分、证据整理',
  category: 'pdf',
  defaultVisible: true,

  routes: [
    {
      path: '/pdf/:tab?',
      name: 'pdf-tools',
      component: () => import('./views/PdfToolsView.vue'),
      meta: { title: 'PDF 工具', moduleId: 'pdf-tools' },
    },
  ],

  menuItems: [
    { label: 'PDF 工具', route: 'pdf-tools', icon: 'Files' },
  ],

  homeCards: [
    {
      title: 'PDF 工具',
      description: '解锁、合并、拆分、证据整理',
      route: 'pdf-tools',
      icon: 'Files',
    },
  ],

  settings: null,
}
