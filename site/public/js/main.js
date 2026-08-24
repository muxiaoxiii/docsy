/* Docsy 下载站交互：数据渲染、平台识别、校验值复制、入场动效 */
(function () {
  'use strict';

  /* 无 JS 或数据加载失败时的回退数据（与发布产物保持一致） */
  var FALLBACK = {
    latest: '0.9.7-beta26',
    githubUrl: 'https://github.com/muxiaoxiii/docsy/releases',
    releases: [
      {
        version: '0.9.7-beta26',
        tag: 'v0.9.7-beta26',
        published_at: '2026-08-24T08:36:57Z',
        assets: [
          {
            platform: 'macos',
            name: 'Docsy_0.9.7-beta26_aarch64.dmg',
            size: 18389199,
            sha256:
              '059dce476491ce3a6ed10c66ca25ec44e1795ad853a9a340a9feff2ea5eef43c',
            url: '/downloads/Docsy_0.9.7-beta26_aarch64.dmg',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta26/Docsy_0.9.7-beta26_aarch64.dmg',
          },
          {
            platform: 'windows',
            name: 'Docsy_0.9.7-beta26_x64-setup.exe',
            size: 12735467,
            sha256:
              '0e4a9b86f7ebde298d27496d86ec39706b565669aadcc99120439ad7a23bcdfb',
            url: '/downloads/Docsy_0.9.7-beta26_x64-setup.exe',
            github_url:
              'https://github.com/muxiaoxiii/docsy/releases/download/v0.9.7-beta26/Docsy_0.9.7-beta26_x64-setup.exe',
          },
        ],
      },
    ],
  };

  var FALLBACK_CHANGELOG = [
    {
      version: '0.9.7-beta26',
      date: '2026-08-24',
      sections: [
        {
          title: '新增',
          items: [
            '**PDF 合并双面打印分隔模式**：开启后，合并时若某份文件页数为奇数，自动在其末尾补一页与最后页同尺寸的空白页，让每份文件都占满双面打印的整张纸，避免两份文件拼在同一张纸的正反面。',
          ],
        },
      ],
    },
    {
      version: '0.9.7-beta25',
      date: '2026-08-18',
      sections: [
        {
          title: '变更',
          items: [
            '**证据处理选项网格对齐统一**：选项网格 min-height 与 justify-content 对齐统一。',
            '**控制栏整合为一行三区**：证据编号、分段与例外、全局应用+开关集中到同一操作栏。',
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

    if (mac) {
      setText('dl-mac-name', mac.name);
      setText('dl-mac-size', formatSize(mac.size));
      setText('dl-mac-sha', mac.sha256 || '');
      setAttr('dl-mac-link', 'href', mac.url || mac.github_url);
      setAttr('dl-mac-github', 'href', mac.github_url);
    }
    if (win) {
      setText('dl-win-name', win.name);
      setText('dl-win-size', formatSize(win.size));
      setText('dl-win-sha', win.sha256 || '');
      setAttr('dl-win-link', 'href', win.url || win.github_url);
      setAttr('dl-win-github', 'href', win.github_url);
    }

    /* Hero CTA 也指向最新版 */
    if (mac) setAttr('cta-macos', 'href', mac.url || mac.github_url);
    if (win) setAttr('cta-windows', 'href', win.url || win.github_url);

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
    items.forEach(function (item) {
      var src = typeof item === 'string' ? item : item.src;
      var alt = typeof item === 'string' ? '' : item.alt || '';
      if (!src) return;
      html +=
        '<figure class="shot-cell">' +
        '<img src="' + escapeHtml(src) + '" alt="' + escapeHtml(alt) + '" loading="lazy" />' +
        (alt ? '<figcaption>' + escapeHtml(alt) + '</figcaption>' : '') +
        '</figure>';
    });
    if (!html) return;
    grid.innerHTML = html;
    section.hidden = false;
    /* 截图区从隐藏变为显示，直接标记内部元素可见 */
    section.querySelectorAll('.reveal').forEach(function (el) {
      el.classList.add('is-visible');
    });
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
