/* Docsy 下载站交互：数据渲染、平台识别、校验值复制、入场动效 */
(function () {
  'use strict';

  /* 无 JS 或数据加载失败时的回退数据（与发布产物保持一致） */
  var FALLBACK = {
    "latest": "1.1.2",
    "githubUrl": "https://github.com/muxiaoxiii/docsy/releases",
    "releases": [
      {
        "version": "1.1.2",
        "tag": "v1.1.2",
        "published_at": "2026-10-23T01:22:43Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.2_aarch64.dmg",
            "size": 55924461,
            "sha256": "59cd73c9a674f90449d41b041d1a8ad42a0991f637e1a5c52860e7e5fc7339b0",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.2_x64-setup.exe",
            "size": 52285365,
            "sha256": "aefdf3b9a4ddfe85296b56d3ac1509b5f11e0c80dcef2b6229619e1d7bd5e34c",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.2_x86-setup.exe",
            "size": 51134416,
            "sha256": "c8afd6d07a825e9c6b8f99b228f182c4a6fc798c6ceb89e137c3c6f17242457b",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.1.1",
        "tag": "v1.1.1",
        "published_at": "2026-10-20T06:02:07Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.1_aarch64.dmg",
            "size": 60149479,
            "sha256": "2dbeaa7cd8f9a773f4858ecd69e338142a48d2a905c1dd1ceb8dcb5cffd4c1a7",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.1/Docsy_1.1.1_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.1/Docsy_1.1.1_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.1_x64-setup.exe",
            "size": 56518616,
            "sha256": "5bea020fdcdca7f60aa34b91b15cf6997ec98871ddfd46d8ee9f4a8412d17b62",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.1/Docsy_1.1.1_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.1/Docsy_1.1.1_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.1_x86-setup.exe",
            "size": 55305523,
            "sha256": "e96d535c949e7ad297d616d3208a9f87cbfb58f8d9dba648ca63434aef68de5a",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.1/Docsy_1.1.1_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.1/Docsy_1.1.1_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.10",
        "tag": "v1.0.10",
        "published_at": "2026-10-19T03:10:48Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.10_aarch64.dmg",
            "size": 49368879,
            "sha256": "512491e7987742e9860a89396ff65734c49334eda20e4b78b7b32413d4277d01",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.10/Docsy_1.0.10_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.10/Docsy_1.0.10_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.10_x64-setup.exe",
            "size": 45877975,
            "sha256": "c576330e9495d64cc9574ebe3e187ec109c8459ca02b58bcc89ed7c5e7a5ac1c",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.10/Docsy_1.0.10_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.10/Docsy_1.0.10_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.10_x86-setup.exe",
            "size": 44686937,
            "sha256": "c1015fcf21a303f51dae85670511f2c7bcc7f00c902a720e8d9062d54766d51b",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.10/Docsy_1.0.10_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.10/Docsy_1.0.10_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.9",
        "tag": "v1.0.9",
        "published_at": "2026-09-24T02:17:53Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.9_aarch64.dmg",
            "size": 49262985,
            "sha256": "fb41a50680d2927649708496b462c0d33b5e071cc5bca951c677eb4ee8162d49",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.9/Docsy_1.0.9_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.9/Docsy_1.0.9_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.9_x64-setup.exe",
            "size": 45789788,
            "sha256": "a547a830743c795f7d0ca57e6302966ae243f1f8578515d8e7b89fd4f7720b62",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.9/Docsy_1.0.9_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.9/Docsy_1.0.9_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.9_x86-setup.exe",
            "size": 44637686,
            "sha256": "1ccf577b887bcdc715db2cf13774a8bb2f04e31e8f07bf5d6d2a4bc407f3ef19",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.9/Docsy_1.0.9_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.9/Docsy_1.0.9_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.8",
        "tag": "v1.0.8",
        "published_at": "2026-09-20T09:17:39Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.8_aarch64.dmg",
            "size": 19962204,
            "sha256": "886e5f096e4d714336a73813b6b97284b541c1eb38caea01f24e8026aeb90bd7",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.8/Docsy_1.0.8_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.8/Docsy_1.0.8_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.8_x64-setup.exe",
            "size": 16175619,
            "sha256": "ba6f07ca170cbc5505e7e4efcce9eb1e5d8c40f339dfae22be0cf5f0364aa385",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.8/Docsy_1.0.8_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.8/Docsy_1.0.8_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.8_x86-setup.exe",
            "size": 14986672,
            "sha256": "3a5663edcca8a8ae810a8bb44b9da3e101fea08ab0352a95680b0ed39400b506",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.8/Docsy_1.0.8_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.8/Docsy_1.0.8_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.7",
        "tag": "v1.0.7",
        "published_at": "2026-09-18T05:07:16Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.7_aarch64.dmg",
            "size": 18497797,
            "sha256": "7ee27bd1eb54b9f6f8ab5e769d051bae08820ca25b11b5a742793c5933493480",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.7/Docsy_1.0.7_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.7/Docsy_1.0.7_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.7_x64-setup.exe",
            "size": 14647613,
            "sha256": "1cf068c016b6fbe32a53a69a8f79d19d2846d51233230e618deb20fd77220d10",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.7/Docsy_1.0.7_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.7/Docsy_1.0.7_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.7_x86-setup.exe",
            "size": 13494745,
            "sha256": "8243e13153e20c836e22f51582a37bf265250056ac362bb93744a8eab658e52e",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.7/Docsy_1.0.7_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.7/Docsy_1.0.7_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.6",
        "tag": "v1.0.6",
        "published_at": "2026-09-17T11:37:39Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.6_aarch64.dmg",
            "size": 18497006,
            "sha256": "93ae61e6fff773c501629e40d3f8a3f738a4da6ae257cd46e27e457097b0fcce",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.6/Docsy_1.0.6_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.6/Docsy_1.0.6_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.6_x64-setup.exe",
            "size": 14636321,
            "sha256": "077dcda5ef9f66ad4a0bc1faa0f28f9fb9a8938092be651faa45e7b843b584d4",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.6/Docsy_1.0.6_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.6/Docsy_1.0.6_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.6_x86-setup.exe",
            "size": 13487285,
            "sha256": "5ff0556e425b926d391555ee08a7a7288919e89e50a7c4ef147ccd19f4cd7682",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.6/Docsy_1.0.6_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.6/Docsy_1.0.6_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.5",
        "tag": "v1.0.5",
        "published_at": "2026-09-16T11:07:06Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.5_aarch64.dmg",
            "size": 18488282,
            "sha256": "d9004776b8d6b0388031c8832ee734239ec1a936183af33feac040618422a223",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.5/Docsy_1.0.5_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.5/Docsy_1.0.5_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.5_x64-setup.exe",
            "size": 14606957,
            "sha256": "5fe8473c4cf427850c3c402360dc27c8f781c3ed80b63d1e8e31ffc661cc294f",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.5/Docsy_1.0.5_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.5/Docsy_1.0.5_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.5_x86-setup.exe",
            "size": 13489388,
            "sha256": "9a0d296fe2b66019fd07da4a57fac0f2637925fd930e1d5097ebc5c998a4c867",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.5/Docsy_1.0.5_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.5/Docsy_1.0.5_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.3",
        "tag": "v1.0.3",
        "published_at": "2026-09-15T10:28:48Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.3_aarch64.dmg",
            "size": 18988139,
            "sha256": "35a80f6672d24303901ade13bbc2d74139de8e891ee7045f437d9c6afce22e85",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.3/Docsy_1.0.3_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.3/Docsy_1.0.3_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.3_x64-setup.exe",
            "size": 15155204,
            "sha256": "a9f5737d0ac7d4d916ce544f28a6d8e782e99f698e5718ea3a30cc69ca680b53",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.3/Docsy_1.0.3_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.3/Docsy_1.0.3_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.3_x86-setup.exe",
            "size": 14005402,
            "sha256": "ef9934fca73115e03a1740cda8351ce0d5fa757ebdaba28aa506d291d6b42503",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.3/Docsy_1.0.3_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.3/Docsy_1.0.3_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.2",
        "tag": "v1.0.2",
        "published_at": "2026-09-14T05:23:41Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.2_aarch64.dmg",
            "size": 19026279,
            "sha256": "b9d280e32125f8518597bf8ca2bd2d4d7540f17b75cd4eef48868de6d1552218",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.2/Docsy_1.0.2_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.2/Docsy_1.0.2_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.2_x64-setup.exe",
            "size": 15072607,
            "sha256": "21f1e7b68eddedfbb964d7e6171e10a2ae99496f061ebeb73609418576a325f0",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.2/Docsy_1.0.2_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.2/Docsy_1.0.2_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.2_x86-setup.exe",
            "size": 13919620,
            "sha256": "97a17b82d0c9ddbef964029481fd90fbf7859368b617be97c619397a131a117e",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.2/Docsy_1.0.2_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.2/Docsy_1.0.2_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.0.1",
        "tag": "v1.0.1",
        "published_at": "2026-09-10T06:39:58Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.0.1_aarch64.dmg",
            "size": 19011803,
            "sha256": "ca39107f852212e14d62543c835b86158e158a333df93e1db7753a6466fe6017",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.0.1_x64-setup.exe",
            "size": 15073369,
            "sha256": "07ee5775d607708f2142e9d8e5465c6505543baafc0c8827abbbbf0371d41a24",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.0.1_x86-setup.exe",
            "size": 13917671,
            "sha256": "b92b2c98644c6d3e519a84184eb0ee4d9e8d2854cda263740e6c70b16d581c24",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_x86-setup.exe"
          }
        ]
      },
      {
        "version": "0.9.7-beta31",
        "tag": "v0.9.7-beta31",
        "published_at": "2026-08-30T16:18:48Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_0.9.7-beta31_aarch64.dmg",
            "size": 18999139,
            "sha256": "9e64027419f2f8535f88eeb87d2cda4bc46fd710a0ab5f7a3566b02f1e06029d",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_0.9.7-beta31_x64-setup.exe",
            "size": 13377303,
            "sha256": "a4d6654fdb3b8353f7bf9220df44224b62c1d118f41b695e05f608208d7441d3",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_0.9.7-beta31_x86-setup.exe",
            "size": 12230948,
            "sha256": "5b0b9905316de532e482adc949698bd057d3b6a8de50cd805597dec7817b861d",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_x86-setup.exe"
          }
        ]
      }
    ]
  };

  var FALLBACK_CHANGELOG = [
    {
      "version": "1.1.2",
      "date": "2026-10-23",
      "sections": [
        {
          "title": "体验与稳定性",
          "items": [
            "Doclet 使用新旧素材补齐八方向，停留后快速转向与回正，修正左上衔接闪帧；点击随机播放动作。",
            "统一按钮、输入框、数字加减、下拉框与提示的尺寸、圆角和对齐；加入按压回弹、侧栏与标签页滑动高亮、弹窗过渡，配置说明收进对应气泡。",
            "重整证据合并设置，页码格式改为下拉并扩充样式，格式预览独立展示；浏览器预览明确提示桌面功能限制。",
            "修正图片排版网格示意、N 字顺序及末页重排，统一手机照片方向标记在预览和导出中的表现。",
            "PDF／证据合并支持 Word 和静态图片；图片默认一图一页，可拖拽合页、排序、旋转、缩放及手动裁剪，调整行列保留手动缩放，保留原始像素导出；修复 Mac Word 自动转换，失败可补充手动 PDF 继续。",
            "证据扫描包含母文件夹文件，支持按文件夹分别输出或全部合并；文件夹内、文件夹间双面补白独立设置，分隔页带识别标记，空白页删除仍由用户勾选并保守检查。",
            "修复启动时的分包依赖问题，公式和图表按需加载，缩减跟随图集与安装资源占用。",
            "修复 Windows 测试启动与工具路径识别，恢复 x64、x86 完整后端回归检查；清理废弃代码与编译警告。"
          ]
        }
      ]
    }
  ];

  function formatSize(bytes) {
    if (!bytes) return '';
    var mb = bytes / 1024 / 1024;
    return (mb >= 100 ? mb.toFixed(0) : mb.toFixed(1)) + ' MB';
  }

  function formatDate(iso) {
    if (!iso) return '';
    var date = new Date(iso);
    if (isNaN(date.getTime())) return iso;
    return date.getFullYear() + '-' + String(date.getMonth() + 1).padStart(2, '0') + '-' + String(date.getDate()).padStart(2, '0');
  }

  /* 渲染 **加粗** 标记为 <strong> */
  function renderBold(text) {
    var parts = String(text).split(/\*\*(.+?)\*\*/g);
    var html = '';
    for (var i = 0; i < parts.length; i++) {
      if (i % 2 === 1) {
        html += '<strong>' + escapeHtml(parts[i]) + '</strong>';
      } else {
        html += escapeHtml(parts[i]);
      }
    }
    return html;
  }

  function escapeHtml(text) {
    return String(text)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function platform() {
    var ua = navigator.userAgent || '';
    if (/Mac/i.test(ua)) return 'macos';
    if (/Windows|Win/i.test(ua)) return 'windows';
    return '';
  }

  function assetFor(release, platformName) {
    var list = (release && release.assets) || [];
    for (var i = 0; i < list.length; i++) {
      if (list[i].platform === platformName) return list[i];
    }
    return null;
  }

  /* ---------- 下载区渲染 ---------- */

  function renderDownloads(releases) {
    if (!releases || !releases.length) return;
    var latest = releases[0];
    var current = platform();

    var mac = assetFor(latest, 'macos');
    var win = assetFor(latest, 'windows');

    var versionEl = document.getElementById('dl-version-static');
    if (versionEl) versionEl.textContent = 'v' + latest.version;

    var macDownload = (mac && mac.github_url) || (mac && mac.url) || FALLBACK.githubUrl;
    var winDownload = (win && win.github_url) || (win && win.url) || FALLBACK.githubUrl;

    if (mac) {
      setText('dl-mac-name', mac.name);
      setText('dl-mac-size', formatSize(mac.size));
      setText('dl-mac-sha', mac.sha256 || '见 GitHub Releases 发布页');
      setAttr('dl-mac-link', 'href', macDownload || '#');
    }
    if (win) {
      setText('dl-win-name', win.name);
      setText('dl-win-size', formatSize(win.size));
      setText('dl-win-sha', win.sha256 || '见 GitHub Releases 发布页');
      setAttr('dl-win-link', 'href', winDownload || '#');
    }

    /* Hero CTA 也指向最新版 */
    if (mac) setAttr('cta-macos', 'href', macDownload || '#');
    if (win) setAttr('cta-windows', 'href', winDownload || '#');
    var x86 = assetFor(latest, 'windows-x86');
    if (x86) setAttr('dl-win-x86', 'href', x86.github_url || x86.url);

    /* 标记当前系统 */
    if (current) {
      var card = document.querySelector('[data-platform-card="' + current + '"]');
      if (card) card.classList.add('is-current');
      var tag = document.querySelector('[data-os-tag="' + current + '"]');
      if (tag) tag.hidden = false;
    }

    setAttr('dl-release-page', 'href', FALLBACK.githubUrl + '/tag/' + (latest.tag || 'v' + latest.version));
    renderHistory(releases);
  }

  function renderHistory(releases) {
    var listEl = document.getElementById('history-list');
    if (!listEl) return;
    listEl.innerHTML = '';
    /* 跳过最新版（已在主卡片里） */
    for (var i = 1; i < releases.length; i++) {
      var release = releases[i];
      var li = document.createElement('li');
      var links = [];
      for (var j = 0; j < release.assets.length; j++) {
        var asset = release.assets[j];
        links.push(
          '<a class="h-dl" href="' + escapeHtml(asset.url || asset.github_url) + '">' +
            (asset.platform === 'macos' ? 'macOS' : asset.platform === 'windows' ? 'Windows 64 位' : asset.platform === 'windows-x86' ? 'Windows 32 位' : asset.platform) +
            '</a>'
        );
      }
      if (!links.length) {
        links.push('<a class="h-dl" href="' + escapeHtml(FALLBACK.githubUrl) + '">查看 GitHub</a>');
      }
      li.innerHTML =
        '<span class="h-version">v' + escapeHtml(release.version) + '</span>' +
        '<span class="h-date">' + escapeHtml(formatDate(release.published_at)) + '</span>' +
        '<span class="h-links">' + links.join('') + '</span>';
      listEl.appendChild(li);
    }
    if (!listEl.children.length) {
      listEl.innerHTML = '<li class="history-empty">暂时只有最新版本，历史版本可前往 GitHub Releases 下载。</li>';
    }
  }

  /* ---------- 更新日志渲染 ---------- */

  function renderChangelog(entries) {
    var listEl = document.getElementById('changelog-list');
    if (!listEl) return;
    listEl.innerHTML = '';
    var top = (entries || []).slice(0, 1);
    if (!top.length) {
      listEl.innerHTML = '<li class="timeline-empty">暂无更新记录。</li>';
      return;
    }
    top.forEach(function (entry) {
      var li = document.createElement('li');
      var head = '<div class="tl-head"><span class="tl-version">v' + escapeHtml(entry.version) + '</span>' +
        '<span class="tl-date">' + escapeHtml(entry.date || '') + '</span></div>';
      var sections = '';
      (entry.sections || []).forEach(function (section) {
        if (!section.items || !section.items.length) return;
        var items = section.items.map(function (item) {
          return '<li>' + renderBold(item) + '</li>';
        }).join('');
        sections +=
          '<div class="tl-section"><div class="tl-section-title">' +
          escapeHtml(section.title) + '</div><ul>' + items + '</ul></div>';
      });
      li.innerHTML = head + '<div class="tl-sections">' + sections + '</div>';
      listEl.appendChild(li);
    });
  }

  function setText(id, value) {
    var el = document.getElementById(id);
    if (el) el.textContent = value;
  }

  function setAttr(id, attr, value) {
    var el = document.getElementById(id);
    if (el && value) el.setAttribute(attr, value);
  }

  /* ---------- 复制校验值 ---------- */

  function initCopyButtons() {
    document.querySelectorAll('.copy-btn').forEach(function (btn) {
      btn.addEventListener('click', function () {
        var selector = btn.getAttribute('data-copy');
        var target = selector ? document.querySelector(selector) : null;
        var text = target ? target.textContent : '';
        if (!text) return;
        var done = function () {
          btn.classList.add('is-copied');
          var original = btn.getAttribute('aria-label') || '';
          btn.setAttribute('aria-label', '已复制');
          setTimeout(function () {
            btn.classList.remove('is-copied');
            btn.setAttribute('aria-label', original);
          }, 1600);
        };
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).then(done).catch(function () {
            legacyCopy(text);
            done();
          });
        } else {
          legacyCopy(text);
          done();
        }
      });
    });
  }

  function legacyCopy(text) {
    var area = document.createElement('textarea');
    area.value = text;
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    try {
      document.execCommand('copy');
    } catch (e) {
      /* ignore */
    }
    document.body.removeChild(area);
  }

  /* ---------- 入场动效与导航 ---------- */

  function initReveal() {
    var elements = document.querySelectorAll('.reveal');
    if (!('IntersectionObserver' in window)) {
      elements.forEach(function (el) {
        el.classList.add('is-visible');
      });
      return;
    }
    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add('is-visible');
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: '0px 0px -8% 0px' }
    );
    elements.forEach(function (el) {
      observer.observe(el);
    });
  }

  function initNav() {
    var nav = document.getElementById('nav');
    if (!nav) return;
    var update = function () {
      nav.classList.toggle('is-scrolled', window.scrollY > 8);
    };
    window.addEventListener('scroll', update, { passive: true });
    update();
  }

  /* ---------- 软件截图 ---------- */

  function renderScreenshots(items) {
    var section = document.getElementById('screenshots');
    var grid = document.getElementById('shot-grid');
    if (!section || !grid || !items || !items.length) return;
    var html = '';
    items.forEach(function (item, index) {
      var src = typeof item === 'string' ? item : item.src;
      var alt = typeof item === 'string' ? '' : item.alt || item.title || '';
      var title = typeof item === 'string' ? '' : item.title || item.alt || '';
      var desc = typeof item === 'string' ? '' : item.description || '';
      if (!src) return;
      var isFeatured = index === 0;
      html +=
        '<div class="shot-item' +
        (isFeatured ? ' shot-featured' : '') +
        ' reveal is-visible">' +
        '<div class="shot-img">' +
        '<img src="' +
        escapeHtml(src) +
        '" alt="' +
        escapeHtml(alt) +
        '" loading="lazy" />' +
        '</div>' +
        '<div class="shot-caption">' +
        '<h4>' +
        escapeHtml(title) +
        '</h4>' +
        (desc ? '<p>' + escapeHtml(desc) + '</p>' : '') +
        '</div>' +
        '</div>';
    });
    if (!html) return;
    grid.innerHTML = html;
    section.hidden = false;
  }

  /* ---------- 数据加载 ---------- */

  function loadJson(url) {
    return fetch(url, { cache: 'no-cache' }).then(function (response) {
      if (!response.ok) throw new Error('HTTP ' + response.status);
      return response.json();
    });
  }

  function init() {
    initCopyButtons();
    initReveal();
    initNav();

    /* 先渲染回退数据，保证首屏可用，再异步刷新 */
    renderDownloads(FALLBACK.releases);
    renderChangelog(FALLBACK_CHANGELOG);

    // Each feed refreshes independently: one missing file must not freeze all data.
    loadJson('data/releases.json').then(function (data) {
      renderDownloads(data.releases || []);
    }).catch(function () {});
    loadJson('data/changelog.json').then(function (data) {
      renderChangelog(data || []);
    }).catch(function () {});
    loadJson('data/screenshots.json').then(function (data) {
      if (data && data.items) renderScreenshots(data.items);
    }).catch(function () {});
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
