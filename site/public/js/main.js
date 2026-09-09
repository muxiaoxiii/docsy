/* Docsy 下载站交互：数据渲染、平台识别、校验值复制、入场动效 */
(function () {
  'use strict';

  /* 无 JS 或数据加载失败时的回退数据（与发布产物保持一致） */
  var FALLBACK = {
    latest: '1.0.1',
    githubUrl: 'https://github.com/muxiaoxiii/docsy/releases',
    releases: [
      {
        version: '1.0.1',
        tag: 'v1.0.1',
        published_at: '2026-09-09T09:45:00Z',
        assets: [
          {
            platform: 'macos',
            name: 'Docsy_1.0.1_aarch64.dmg',
            size: 19205444,
            sha256: 'd1020790737e56d2642288f973804119f6d418362e21c9ef9c96dccabbee7432',
            url: 'https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_aarch64.dmg',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_aarch64.dmg',
          },
          {
            platform: 'windows',
            name: 'Docsy_1.0.1_x64-setup.exe',
            size: 12735467,
            sha256: '',
            url: 'https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_x64-setup.exe',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v1.0.1/Docsy_1.0.1_x64-setup.exe',
          },
        ],
      },
      {
        version: '0.9.7-beta31',
        tag: 'v0.9.7-beta31',
        published_at: '2026-08-30T10:00:00Z',
        assets: [
          {
            platform: 'macos',
            name: 'Docsy_0.9.7-beta31_aarch64.dmg',
            size: 18389199,
            sha256: '',
            url: 'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_aarch64.dmg',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_aarch64.dmg',
          },
          {
            platform: 'windows',
            name: 'Docsy_0.9.7-beta31_x64-setup.exe',
            size: 12735467,
            sha256: '',
            url: 'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_x64-setup.exe',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta31/Docsy_0.9.7-beta31_x64-setup.exe',
          },
        ],
      },
      {
        version: '0.9.7-beta30',
        tag: 'v0.9.7-beta30',
        published_at: '2026-08-30T06:00:00Z',
        assets: [
          {
            platform: 'macos',
            name: 'Docsy_0.9.7-beta30_aarch64.dmg',
            size: 18389199,
            sha256: '',
            url: 'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta30/Docsy_0.9.7-beta30_aarch64.dmg',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta30/Docsy_0.9.7-beta30_aarch64.dmg',
          },
          {
            platform: 'windows',
            name: 'Docsy_0.9.7-beta30_x64-setup.exe',
            size: 12735467,
            sha256: '',
            url: 'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta30/Docsy_0.9.7-beta30_x64-setup.exe',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta30/Docsy_0.9.7-beta30_x64-setup.exe',
          },
        ],
      },
    ],
  };

  var FALLBACK_CHANGELOG = [
    {
      version: '1.0.1',
      date: '2026-09-09',
      sections: [
        {
          title: '新增与优化',
          items: [
            '**首次运行引导与组件一键配置向导**：新增组件配置向导弹窗，支持用户一键部署外部依赖工具。',
            '**纯血内置 Homebrew 极速部署**：免交互免确认，基于中科大/清华镜像 CDN 秒级拉取部署，杜绝等待与问卷。',
            '**设置页环境检测强化**：外部工具状态区加入高亮向导唤起入口与未就绪组件智能横幅。',
            '**终端与多平台通路自愈**：解决多行终端脚本截断与非空目录拉取报错，安装完毕自动关闭窗口。',
          ],
        },
      ],
    },
    {
      version: '1.0.0',
      date: '2026-09-08',
      sections: [
        {
          title: '里程碑',
          items: [
            '**Docsy 1.0.0 正式定版**：首个正式稳定版，提供纯本地、零网络上传的安全文档生产力体验。',
            '**文书模板工程**：基于 quick-xml 精确解析，支持 Word 标黄自动建模、8 种字段类型、智能上下文回填与 Excel 批量渲染。',
            '**证据处理工作台**：三层页眉页脚智能检测、证据编号与页码规则编排、页段边界智能衔接与合并拆分、空白页自动移除。',
            '**媒体与排版工具箱**：智能抽帧选图工作流、OpenCV 画面分析、自适应 A4 图片排版、PDF 基础工具与 Markdown/Office 互转。',
          ],
        },
      ],
    },
    {
      version: '0.9.7-beta31',
      date: '2026-08-30',
      sections: [
        {
          title: '新增',
          items: [
            '**合并证据拆分优化**：完善证据标签全局识别与按证据边界自动分组，合并导入支持快速重新检测、撤销/重做与“合并到上一段”。',
            '**PDF 拆分与证据工作台预览交互**：支持自由拖拽分割条调整预览宽度、“从本页拆”与“从上一页拆”快捷操作、以及带防抖的下一页缩略图实时预览。',
            '**全局导航收缩**：支持左侧主导航栏展开/收缩切换，收缩后仅显示图标，为工作区与预览区提供更大展示空间。',
          ],
        },
      ],
    },
    {
      version: '0.9.7-beta30',
      date: '2026-08-30',
      sections: [
        {
          title: '修复',
          items: [
            '**全项目可靠性修复**：集中修复外部命令管道阻塞、任务取消、解压边界、PDF 与 DOCX 处理、前端状态恢复等代码审查发现的问题。',
            '**合并证据自动拆分**：支持识别合并文件任意页的证据标签，并限制普通页眉必须跨页重复且位置稳定，避免正文被误判为拆分点。',
          ],
        },
      ],
    },
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
    return list[0] || null;
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
      setAttr('dl-mac-github', 'href', mac.github_url);
    }
    if (win) {
      setText('dl-win-name', win.name);
      setText('dl-win-size', formatSize(win.size));
      setText('dl-win-sha', win.sha256 || '见 GitHub Releases 发布页');
      setAttr('dl-win-link', 'href', winDownload || '#');
      setAttr('dl-win-github', 'href', win.github_url);
    }

    /* Hero CTA 也指向最新版 */
    if (mac) setAttr('cta-macos', 'href', macDownload || '#');
    if (win) setAttr('cta-windows', 'href', winDownload || '#');

    /* 标记当前系统 */
    if (current) {
      var card = document.querySelector('[data-platform-card="' + current + '"]');
      if (card) card.classList.add('is-current');
      var tag = document.querySelector('[data-os-tag="' + current + '"]');
      if (tag) tag.hidden = false;
    }

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
            (asset.platform === 'macos' ? 'macOS' : asset.platform === 'windows' ? 'Windows' : asset.platform) +
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
    var top = (entries || []).slice(0, 5);
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

    Promise.all([
      loadJson('data/releases.json'),
      loadJson('data/changelog.json'),
      loadJson('data/screenshots.json').catch(function () {
        return null;
      }),
    ])
      .then(function (results) {
        renderDownloads(results[0].releases || []);
        renderChangelog(results[1] || []);
        if (results[2] && results[2].items) renderScreenshots(results[2].items);
      })
      .catch(function () {
        /* 回退数据已在首屏渲染，静默即可 */
      });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
