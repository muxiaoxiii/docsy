export default {
  id: 'video-extract',
  name: '视频抽帧',
  icon: 'VideoCamera',
  description: '按时间或频率导出视频帧',
  category: 'media',
  order: 50,
  defaultVisible: true,

  routes: [
    {
      path: '/video-extract',
      name: 'video-extract',
      component: () => import('./views/VideoExtractView.vue'),
      meta: { title: '视频抽帧', moduleId: 'video-extract' },
    },
  ],

  menuItems: [{ label: '视频抽帧', route: 'video-extract', icon: 'VideoCamera' }],

  homeCards: [
    {
      title: '视频抽帧',
      description: '按时间或频率导出视频帧',
      route: 'video-extract',
      icon: 'VideoCamera',
    },
  ],

  settings: null,
}
