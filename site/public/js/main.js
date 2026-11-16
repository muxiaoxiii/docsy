/* Docsy 下载站交互：数据渲染、平台识别、校验值复制、入场动效 */
(function () {
  'use strict';

  /* 无 JS 或数据加载失败时的回退数据（与发布产物保持一致） */
  var FALLBACK = {
    "latest": "1.1.6",
    "githubUrl": "https://github.com/muxiaoxiii/docsy/releases",
    "releases": [
      {
        "version": "1.1.6",
        "tag": "v1.1.6",
        "published_at": "2026-11-12T05:02:29Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.6_aarch64.dmg",
            "size": 59433100,
            "sha256": "9df42b88d32727b7a9ea86aeb17ff136d0468bab18109285b99266c2a2499639",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.6_x64-setup.exe",
            "size": 53645511,
            "sha256": "0a16d5343fa73c8f5ee07791f26a4020a5153ff4884a255adf0618cf3982ddc2",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_x64-setup.exe"
          },
          {
            "platform": "macos-intel",
            "name": "Docsy_1.1.6_x64.dmg",
            "size": 60565781,
            "sha256": "3958dc2397cbaed3d581007c158412f99c05c2beea69565c916c4fa2f34528a3",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_x64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_x64.dmg"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.6_x86-setup.exe",
            "size": 52493944,
            "sha256": "962951cd78a7ebeff0a20c428f2de3638e5092a054c4bffa7200e73c1e296281",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.6/Docsy_1.1.6_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.1.5",
        "tag": "v1.1.5",
        "published_at": "2026-11-06T09:20:57Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.5_aarch64.dmg",
            "size": 57833491,
            "sha256": "1369b7a4d578bf5c4788d1d0e7edbad6937397593dce7a397484db7b18e9f0fb",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.5_x64-setup.exe",
            "size": 53634658,
            "sha256": "4d87fa0216bef136e6eb03169de7b8b2739e402f2faeddf08dc3b28b4bc7b691",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_x64-setup.exe"
          },
          {
            "platform": "macos-intel",
            "name": "Docsy_1.1.5_x64.dmg",
            "size": 60496228,
            "sha256": "00886c94e2646dd41f63d3d4c1411ab41156fc5ee5d64bfb99cc76a0b403fbc1",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_x64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_x64.dmg"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.5_x86-setup.exe",
            "size": 52396351,
            "sha256": "545f83783ce4f87118e8d59fffe75d38c331e250e6d1abab780d6dd3e001710a",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.5/Docsy_1.1.5_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.1.4",
        "tag": "v1.1.4",
        "published_at": "2026-11-03T03:56:54Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.4_aarch64.dmg",
            "size": 57722034,
            "sha256": "31b640b9dbafcc99b0f17449e987e6b665464cc7ccd6bb74e7172b9808e80330",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.4_x64-setup.exe",
            "size": 53601299,
            "sha256": "83e3ae4cb970b8173210a42050af0ff299a0c68a73fba6a0e41b990259183334",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x64-setup.exe"
          },
          {
            "platform": "macos-intel",
            "name": "Docsy_1.1.4_x64.dmg",
            "size": 60397189,
            "sha256": "44bc9913cdabfdafaebaae844f2c53c398c393fa706295f4b919c32e2bca1105",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x64.dmg"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.4_x86-setup.exe",
            "size": 52369918,
            "sha256": "00c2372c24de95f9448ef1f343266cf75da011383e94f61390dfa36e7e6d210e",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.1.3",
        "tag": "v1.1.3",
        "published_at": "2026-11-02T03:47:50Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.3_aarch64.dmg",
            "size": 55937224,
            "sha256": "0ae6a11ae7dddea52fc7cb714c9934c88f75a62a38954669e50e287e327691cb",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.3_x64-setup.exe",
            "size": 52337043,
            "sha256": "5b1a58b82cc3f57b0283811ff5f826fd08ab1f25b9bd6a1f131a282c09ea9925",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.3_x86-setup.exe",
            "size": 51172141,
            "sha256": "585836e6672430ecd67318a455e5119c13aa7625ca9984abacfdded02ce9e410",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.1.2",
        "tag": "v1.1.2",
        "published_at": "2026-10-23T01:22:43Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.2_aarch64.dmg",
            "size": 55914400,
            "sha256": "24ecc3a02d986fc85faf67ded7a5eb307eb9d4744fdf1d9e47174287d7e27341",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.2_x64-setup.exe",
            "size": 52293013,
            "sha256": "1ce308b1c56316000ffe99ca741d9d8e8c44ddf9acfff26dda9dbde245e73c98",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.2_x86-setup.exe",
            "size": 51138434,
            "sha256": "c4637c3cae58de03d95dcc87256f8d105af7fe32183badc4de602f0793c4a390",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x86-setup.exe"
          }
        ]
      }
    ]
  };

  var FALLBACK_CHANGELOG = [
    {
      "version": "1.1.7",
      "date": "2026-11-16",
      "sections": [
        {
          "title": "改进与修复",
          "items": [
            "修复原页眉页脚检测中，页眉里的页码能被识别却删不掉的问题。删除目标此前按元素类型分流，而页码在页眉和页脚都会出现，页眉页码被当成页脚目标下发，后端按区域匹配时永远命中不了。现改为按页眉/页脚区域分流，历史存档自动按位置归位；删除页眉页码时只移除文字本身，同一文本块内用相对位移绘制的页脚与正文位置不变。",
            "修正证据工作台「检测现有页眉页脚」按钮比同一行的其他操作按钮大一号、文字溢出的问题。"
          ]
        }
      ]
    },
    {
      "version": "1.1.6",
      "date": "2026-11-12",
      "sections": [
        {
          "title": "新增与改进",
          "items": [
            "证据合并新增 Word 证据目录导出，按文件顺序填写证据名称及所选的页码范围。",
            "支持手动按当前列表导出；合并成功后自动按实际输出结果生成目录。双面分隔可选择空白页是否计入连续页码。"
          ]
        }
      ]
    },
    {
      "version": "1.1.5",
      "date": "2026-11-06",
      "sections": [
        {
          "title": "改进与修复",
          "items": [
            "修复证据批注清理误删印章及遗留关联评论的问题；保留印章与签名外观，显示实际删除数量和未发现批注的结果。",
            "补齐证据合并、证据拆分、证据扫描和图片等工作列表的“清空列表”入口，同步重置预览与结果，处理时禁用且不删除磁盘文件。",
            "新增 PDF 页面裁剪，支持按页面范围应用，并可将结果送往 PDF 合并或页面提取。",
            "修复长图输出图片被过度降采样，统一 PDF 预览和导出的切分位置，增加相邻页对照与扫描图切线判断。",
            "统一文档处理结果的“继续处理”入口，按格式提供兼容工具；保留跨模块工作状态，导入失败可重试。",
            "收敛 SVG 场景动效，修正短窗口下 MD 操作栏显示；证据合并先导入文件再展示处理选项。"
          ]
        }
      ]
    },
    {
      "version": "1.1.4",
      "date": "2026-11-02",
      "sections": [
        {
          "title": "新增功能与体验优化",
          "items": [
            "新增「长图处理」功能：支持网页长截图、手机长截屏图片（PNG/JPG/WebP/BMP）以及单页或多页 PDF 逐页智能切片，合并输出标准 A4 打印文件；多页预览标明来源页码。",
            "智能切刀防截断：通过视觉空白间隙与色差方差检测，自动寻找自然段落留白处下刀，避免文字行被截断或腰斩。",
            "多维度排版适配：支持 A4 纵向/横向切换、四周边距调节、内容宽度模式（满版 / 手机适中 55% / 自定义比例）与水平对齐设置。",
            "页眉与页脚定制：支持添加来源网址或标题、打印日期（可自定义是否显示“打印日期：”前缀）及标准多样式页脚页码。",
            "实时 A4 打印预览：提供 A4 纸张模拟翻页预览，所见即所得；支持拖拽批量处理，并融入 Doclet 动效与完成庆祝反馈。",
            "依赖与兼容性：升级 PdfCraft v0.18.0、Mermaid 12 及 Tauri/Rust 安全依赖，适配 quick-xml 新版实体引用解析。"
          ]
        }
      ]
    },
    {
      "version": "1.1.3",
      "date": "2026-11-02",
      "sections": [
        {
          "title": "改进与修复",
          "items": [
            "优化证据拆分：支持批量分段、精确跳转首尾页和预览缓存，改进页眉与页码相邻时的识别。",
            "统一证据合并的页眉、页脚和页码配置、列表及预览；支持逐文件编辑、文件名选字和全局修改冲突提示与撤销。",
            "改进 PDF 页面裁切、旋转和页面尺寸处理，修复部分文件预览及原页眉页脚定位。",
            "改进 PDF 合并输出目录和文件名设置；同名输出自动加序号，不覆盖原文件。",
            "调整证据设置及文件列表布局，改善按钮对齐和窄窗口下的显示。"
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
    var intel = assetFor(latest, 'macos-intel');
    if (intel) setAttr('dl-mac-intel', 'href', intel.github_url || intel.url);

    /* 标记当前系统 */
    if (current) {
      var card = document.querySelector('[data-platform-card="' + current + '"]');
      if (card) card.classList.add('is-current');
      var tag = document.querySelector('[data-os-tag="' + current + '"]');
      if (tag) tag.hidden = false;
    }

    setAttr('dl-release-page', 'href', FALLBACK.githubUrl + '/tag/' + (latest.tag || 'v' + latest.version));
  }


  /* ---------- 更新日志渲染 ---------- */

  var changelogEntries = [];
  var changelogExpanded = false;

  function renderChangelog(entries) {
    if (entries && entries.length) changelogEntries = entries;
    var listEl = document.getElementById('changelog-list');
    if (!listEl) return;
    listEl.innerHTML = '';
    var list = changelogExpanded ? changelogEntries : changelogEntries.slice(0, 5);
    if (!list.length) {
      listEl.innerHTML = '<li class="timeline-empty">暂无更新记录。</li>';
      return;
    }
    list.forEach(function (entry) {
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

    var toggleBtn = document.getElementById('changelog-toggle-btn');
    if (toggleBtn) {
      if (changelogEntries.length > 5) {
        toggleBtn.hidden = false;
        toggleBtn.textContent = changelogExpanded ? '收起历史版本' : '展开更多历史版本 (' + changelogEntries.length + ')';
      } else {
        toggleBtn.hidden = true;
      }
    }
  }

  function initChangelogToggle() {
    var toggleBtn = document.getElementById('changelog-toggle-btn');
    if (!toggleBtn) return;
    toggleBtn.addEventListener('click', function () {
      changelogExpanded = !changelogExpanded;
      renderChangelog();
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
    initChangelogToggle();

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
