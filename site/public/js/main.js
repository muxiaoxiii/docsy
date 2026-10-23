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
