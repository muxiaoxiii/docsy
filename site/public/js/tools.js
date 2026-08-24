/* 外部工具下载页：从 data/tools.json 渲染工具卡片（含回退数据） */
(function () {
  'use strict';

  var FALLBACK = {
    tools: {
      qpdf: {
        label: 'qpdf',
        purpose: 'PDF 合并、拆分、叠加和结构处理',
        page: 'https://github.com/qpdf/qpdf/releases',
        entries: [
          {
            platform: 'macos',
            os: 'macOS（Apple 芯片 / Intel）',
            type: 'brew',
            version: 'Homebrew 最新版',
            command: 'brew install qpdf',
            url: 'https://formulae.brew.sh/formula/qpdf',
            size: null,
            sha256: '',
            note: '推荐方式：Docsy 会自动检测系统已安装的 qpdf',
          },
          {
            platform: 'windows',
            os: 'Windows 10 / 11（64 位）',
            type: 'zip',
            version: '12.3.2',
            command: '',
            url: 'https://github.com/qpdf/qpdf/releases/download/v12.3.2/qpdf-12.3.2-msvc64.zip',
            size: 24536601,
            sha256:
              '8941870a604e7c87ed24566b038d46c24ce76616254d2383c578f60c0677f202',
            mirror_url:
              'https://gh-proxy.com/https://github.com/qpdf/qpdf/releases/download/v12.3.2/qpdf-12.3.2-msvc64.zip',
            note: '官方 msvc64 压缩包；也可以在 Docsy 设置页直接「下载安装到 Docsy」自动安装',
          },
        ],
      },
      poppler: {
        label: 'Poppler',
        purpose: 'PDF 预览渲染和页眉页脚文本检测',
        page: 'https://github.com/oschwartz10612/poppler-windows/releases',
        entries: [
          {
            platform: 'macos',
            os: 'macOS（Apple 芯片 / Intel）',
            type: 'brew',
            version: 'Homebrew 最新版',
            command: 'brew install poppler',
            url: 'https://formulae.brew.sh/formula/poppler',
            size: null,
            sha256: '',
            note: '推荐方式：Docsy 会自动检测系统已安装的 Poppler',
          },
          {
            platform: 'windows',
            os: 'Windows 10 / 11（64 位）',
            type: 'zip',
            version: '26.02.0-0',
            command: '',
            url: 'https://github.com/oschwartz10612/poppler-windows/releases/download/v26.02.0-0/Release-26.02.0-0.zip',
            size: 16138240,
            sha256:
              '993e4a94376ed712fafc7058d724ea0b943d118bbd2305cd9ed55174eb85cda5',
            mirror_url:
              'https://gh-proxy.com/https://github.com/oschwartz10612/poppler-windows/releases/download/v26.02.0-0/Release-26.02.0-0.zip',
            note: 'Poppler 社区 Windows 打包（官方推荐）；也可以让 Docsy 自动安装',
          },
        ],
      },
      ffmpeg: {
        label: 'FFmpeg',
        purpose: '视频信息读取、抽帧和时间戳水印',
        page: 'https://github.com/BtbN/FFmpeg-Builds/releases',
        entries: [
          {
            platform: 'macos',
            os: 'macOS（Apple 芯片 / Intel）',
            type: 'brew',
            version: 'Homebrew 最新版',
            command: 'brew install ffmpeg',
            url: 'https://formulae.brew.sh/formula/ffmpeg',
            size: null,
            sha256: '',
            note: '时间戳水印需要 drawtext 滤镜，Homebrew 官方 ffmpeg 已包含；Docsy 会自动检测',
          },
          {
            platform: 'windows',
            os: 'Windows 10 / 11（64 位）',
            type: 'zip',
            version: '9.0（master 滚动版）',
            command: '',
            url: 'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip',
            size: 170641785,
            sha256: '',
            mirror_url:
              'https://gh-proxy.com/https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip',
            note: 'BtbN 官方构建（GPL 版，含 drawtext 滤镜）；滚动更新无固定校验值，也可以让 Docsy 自动安装',
          },
        ],
      },
    },
  };

  function escapeHtml(text) {
    return String(text)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function formatSize(bytes) {
    if (!bytes) return '';
    var mb = bytes / 1024 / 1024;
    return (mb >= 100 ? mb.toFixed(0) : mb.toFixed(1)) + ' MB';
  }

  var ICONS = {
    apple:
      '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8.286 7.008c-3.216 0 -4.286 3.23 -4.286 5.92c0 3.229 2.143 8.072 4.286 8.072c1.165 -.05 1.799 -.538 3.214 -.538c1.406 0 1.607 .538 3.214 .538s2.326 -1.074 3.214 -1.074c1.28 0 1.895 1.074 3.214 1.074c1.406 0 2.326 -.538 3.214 -.538c1.58 -.074 3.213 -4.736 3.213 -4.736c-3.214 -.215 -4.016 -3.5 -1.071 -4.986" /><path d="M14.292 3.258c.577 -.748 1.605 -1.346 2.444 -1.367c.128 1.088 -.342 2.15 -1.051 2.89c-.697 .752 -1.816 1.325 -2.635 1.19c-.141 -1.026 .378 -2.081 1.242 -2.713" /></svg>',
    windows:
      '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17.8 20l-12 -1.5c-1 -.1 -1.8 -.9 -1.8 -1.9v-9.2c0 -1 .8 -1.8 1.8 -1.9l12 -1.5c1.2 -.1 2.2 .8 2.2 1.9v12.1c0 1.1 -.9 2 -2.2 1.9" /><path d="M12 5l0 14" /><path d="M4 12l16 0" /></svg>',
    download:
      '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2 -2v-2" /><path d="M7 11l5 5l5 -5" /><path d="M12 4l0 12" /></svg>',
    copy:
      '<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 9.667a2.667 2.667 0 0 1 2.667 -2.667h8.666a2.667 2.667 0 0 1 2.667 2.667v8.666a2.667 2.667 0 0 1 -2.667 2.667h-8.666a2.667 2.667 0 0 1 -2.667 -2.667z" /><path d="M4.012 16.737a2 2 0 0 1 -1.012 -1.737v-10c0 -1.1 .9 -2 2 -2h10c.75 0 1.158 .385 1.5 1" /></svg>',
    check:
      '<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12l5 5l10 -10" /></svg>',
    external:
      '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 6h-6a2 2 0 0 0 -2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2 -2v-6" /><path d="M11 13l9 -9" /><path d="M15 4h5v5" /></svg>',
  };

  function entryRow(entry) {
    var osIcon = entry.platform === 'macos' ? ICONS.apple : ICONS.windows;
    var download = '';
    if (entry.type === 'brew') {
      download =
        '<a class="btn btn-primary btn-sm" href="' + escapeHtml(entry.url) + '" target="_blank" rel="noopener">' +
        ICONS.download + ' Homebrew 公式</a>' +
        '<code class="code-inline">' + escapeHtml(entry.command) + '</code>';
    } else {
      var hashRow = '';
      if (entry.sha256) {
        hashRow =
          '<div class="tools-hash"><span class="dl-hash-label">SHA256</span>' +
          '<code>' + escapeHtml(entry.sha256) + '</code>' +
          '<button class="copy-btn" type="button" data-copy-text="' + escapeHtml(entry.sha256) + '" aria-label="复制 SHA256 校验值">' + ICONS.copy + '</button></div>';
      }
      download =
        '<div class="tools-dl"><a class="btn btn-primary btn-sm" href="' + escapeHtml(entry.url) + '" target="_blank" rel="noopener">' +
        ICONS.download + ' 官方下载 ' + escapeHtml(formatSize(entry.size)) + '</a>' +
        (entry.mirror_url
          ? '<a class="dl-alt" href="' + escapeHtml(entry.mirror_url) + '" target="_blank" rel="noopener">国内加速镜像</a>'
          : '') +
        '</div>' + hashRow;
    }
    return (
      '<div class="tools-entry">' +
      '<div class="tools-os">' + osIcon + '<span>' + escapeHtml(entry.os) + '</span></div>' +
      '<div class="tools-ver"><span class="tools-ver-label">版本</span><strong>' + escapeHtml(entry.version) + '</strong></div>' +
      '<div class="tools-act">' + download + '</div>' +
      '<p class="tools-note">' + escapeHtml(entry.note) + '</p>' +
      '</div>'
    );
  }

  function render(toolsData) {
    var listEl = document.getElementById('tool-list');
    if (!listEl) return;
    var tools = (toolsData && toolsData.tools) || {};
    var html = '';
    Object.keys(tools).forEach(function (name) {
      var tool = tools[name];
      if (!tool) return;
      html +=
        '<article class="tool-card">' +
        '<header class="tool-card-head">' +
        '<div><h2>' + escapeHtml(tool.label) + '</h2><p>' + escapeHtml(tool.purpose) + '</p></div>' +
        '<a class="dl-alt" href="' + escapeHtml(tool.page || '') + '" target="_blank" rel="noopener">官方发布页 ' + ICONS.external + '</a>' +
        '</header>' +
        '<div class="tools-entries">' +
        (tool.entries || []).map(entryRow).join('') +
        '</div>' +
        '</article>';
    });
    listEl.innerHTML = html || '<div class="tool-card"><p class="tools-note">暂无工具信息。</p></div>';
    initCopyButtons(listEl);
    initReveal();
  }

  function initCopyButtons(scope) {
    scope.querySelectorAll('.copy-btn').forEach(function (btn) {
      btn.addEventListener('click', function () {
        var text = btn.getAttribute('data-copy-text') || '';
        if (!text) return;
        var done = function () {
          btn.classList.add('is-copied');
          btn.innerHTML = ICONS.check;
          setTimeout(function () {
            btn.classList.remove('is-copied');
            btn.innerHTML = ICONS.copy;
          }, 1600);
        };
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).then(done).catch(done);
        } else {
          done();
        }
      });
    });
  }

  function initReveal() {
    document.querySelectorAll('.reveal').forEach(function (el) {
      el.classList.add('is-visible');
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

  render(FALLBACK);

  fetch('data/tools.json', { cache: 'no-cache' })
    .then(function (response) {
      if (!response.ok) throw new Error('HTTP ' + response.status);
      return response.json();
    })
    .then(render)
    .catch(function () {
      /* 回退数据已渲染 */
    });

  initNav();
})();
