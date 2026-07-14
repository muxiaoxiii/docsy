export default {
  id: 'evidence-pdf',
  name: '证据处理',
  icon: 'DocumentChecked',
  description: '证据 PDF 的合并、拆分、页眉页码和扫描',
  category: 'pdf',
  order: 20,
  defaultVisible: true,

  routes: [
    {
      path: '/evidence',
      name: 'evidence-pdf',
      component: () => import('./views/EvidencePdfView.vue'),
      meta: { title: '证据处理', moduleId: 'evidence-pdf' },
    },
  ],

  menuItems: [
    { label: '证据处理', route: 'evidence-pdf', icon: 'DocumentChecked' },
  ],

  homeCards: [
    {
      title: '证据处理',
      description: '证据合并、证据拆分、证据扫描',
      route: 'evidence-pdf',
      icon: 'DocumentChecked',
    },
  ],

  settings: null,
}
