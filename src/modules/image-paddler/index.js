export default {
  id: 'image-paddler',
  name: '图片排版',
  icon: 'Picture',
  description: '图片批量排版为 A4 文档',
  category: 'media',
  defaultVisible: true,

  routes: [
    {
      path: '/image-paddler',
      name: 'image-paddler',
      component: () => import('./views/ImagePaddlerView.vue'),
      meta: { title: '图片排版', moduleId: 'image-paddler' },
    },
  ],

  menuItems: [
    { label: '图片排版', route: 'image-paddler', icon: 'Picture' },
  ],

  homeCards: [
    {
      title: '图片排版',
      description: '图片批量排版为 A4 文档',
      route: 'image-paddler',
      icon: 'Picture',
    },
  ],

  settings: null,
}
