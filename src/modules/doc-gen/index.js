export default {
  id: 'doc-gen',
  name: '文档生成',
  icon: 'Document',
  description: '基于模板生成 Word/PDF 文档',
  category: 'document',
  defaultVisible: true,

  routes: [
    {
      path: '/doc-gen/:templateId?',
      name: 'doc-gen-form',
      component: () => import('./views/GenFormView.vue'),
      meta: { title: '文档生成', moduleId: 'doc-gen' },
    },
    {
      path: '/doc-gen/batch',
      name: 'doc-gen-batch',
      component: () => import('./views/GenBatchView.vue'),
      meta: { title: '批量生成', moduleId: 'doc-gen' },
    },
    {
      path: '/doc-gen/records',
      name: 'doc-gen-records',
      component: () => import('./views/GenRecordsView.vue'),
      meta: { title: '记录中心', moduleId: 'doc-gen' },
    },
  ],

  menuItems: [
    { label: '文档生成', route: 'doc-gen-form', icon: 'Document' },
    { label: '批量生成', route: 'doc-gen-batch', icon: 'DocumentCopy' },
    { label: '记录中心', route: 'doc-gen-records', icon: 'Clock' },
  ],

  homeCards: [
    {
      title: '生成文档',
      description: '选择模板，填写表单，一键生成',
      route: 'doc-gen-form',
      icon: 'Document',
    },
  ],

  settings: null,
}
