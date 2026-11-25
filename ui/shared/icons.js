/* 아이콘 (16px 선 아이콘, currentColor) 과 에이전트 표시 */
(function () {
  'use strict';
  const svg = (d, extra = '') => `<svg class="ic" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" ${extra}>${d}</svg>`;

  const I = {
    overview: svg('<rect x="2" y="2" width="5" height="5" rx="1.2"/><rect x="9" y="2" width="5" height="5" rx="1.2"/><rect x="2" y="9" width="5" height="5" rx="1.2"/><rect x="9" y="9" width="5" height="5" rx="1.2"/>'),
    library: svg('<path d="M3 2.5v11M6 2.5v11"/><path d="M8.6 3.2l2.6-.7 2.6 10.3-2.6.7z"/><path d="M2 13.5h12"/>'),
    targets: svg('<path d="M2 4.5h4l1.2 1.5H14v6.5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/><path d="M8 8.2v3M6.5 9.7 8 11.2l1.5-1.5"/>'),
    look: svg('<circle cx="8" cy="8" r="6"/><path d="M8 2a6 6 0 0 0 0 12z" fill="currentColor"/>'),
    bell: svg('<path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.5H3z"/><path d="M6.5 14a1.6 1.6 0 0 0 3 0"/>'),
    gear: svg('<circle cx="8" cy="8" r="2.2"/><path d="M8 1.8v1.6M8 12.6v1.6M14.2 8h-1.6M3.4 8H1.8M12.4 3.6l-1.1 1.1M4.7 11.3l-1.1 1.1M12.4 12.4l-1.1-1.1M4.7 4.7 3.6 3.6"/>'),
    refresh: svg('<path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9"/><path d="M13.5 2.5v3h-3"/>'),
    min: svg('<path d="M3.5 8h9"/>'),
    max: svg('<rect x="3.5" y="3.5" width="9" height="9" rx="1"/>'),
    close: svg('<path d="M4 4l8 8M12 4l-8 8"/>'),
    search: svg('<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5 14 14"/>'),
    plus: svg('<path d="M8 3v10M3 8h10"/>'),
    folder: svg('<path d="M2 4.5h4l1.2 1.5H14v6.5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>'),
    zip: svg('<path d="M4 1.8h5.5L13 5.3V14a.5.5 0 0 1-.5.5h-8.5A.5.5 0 0 1 3.5 14V2.3a.5.5 0 0 1 .5-.5z"/><path d="M7 3v1M7 5.5v1M7 8v1"/><rect x="6" y="10" width="2" height="2.2" rx=".4"/>'),
    download: svg('<path d="M8 2.5v8M4.8 7.4 8 10.6l3.2-3.2"/><path d="M2.5 13.5h11"/>'),
    upload: svg('<path d="M8 10.5v-8M4.8 5.6 8 2.4l3.2 3.2"/><path d="M2.5 13.5h11"/>'),
    trash: svg('<path d="M2.5 4h11M6 4V2.5h4V4M4 4l.7 9.5h6.6L12 4"/>'),
    copy: svg('<rect x="5" y="5" width="8.5" height="8.5" rx="1.2"/><path d="M3 10.5V3a.5.5 0 0 1 .5-.5H11"/>'),
    check: svg('<path d="M3 8.5 6.5 12 13 4.5"/>'),
    alert: svg('<path d="M8 2 14.5 13.5h-13z"/><path d="M8 6.5v3M8 11.6v.1"/>'),
    info: svg('<circle cx="8" cy="8" r="6"/><path d="M8 7.2v4M8 4.9v.1"/>'),
    back: svg('<path d="M10 3 5 8l5 5"/>'),
    star: svg('<path d="m8 2 1.8 3.8 4.2.5-3.1 2.9.8 4.1L8 11.3l-3.7 2 .8-4.1L2 6.3l4.2-.5z"/>'),
    starOn: svg('<path d="m8 2 1.8 3.8 4.2.5-3.1 2.9.8 4.1L8 11.3l-3.7 2 .8-4.1L2 6.3l4.2-.5z" fill="currentColor"/>'),
    edit: svg('<path d="M10.5 2.5 13.5 5.5 5.5 13.5H2.5V10.5z"/>'),
    file: svg('<path d="M4 1.8h5.5L13 5.3V14a.5.5 0 0 1-.5.5h-8.5A.5.5 0 0 1 3.5 14V2.3a.5.5 0 0 1 .5-.5z"/><path d="M9.5 1.8v3.5H13"/>'),
    link: svg('<path d="M9 3h4v4M13 3 7.5 8.5"/><path d="M11.5 9.5v3a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1h3"/>'),
    sync: svg('<path d="M2.5 6.5A5.5 5.5 0 0 1 12.6 4.5M13.5 9.5A5.5 5.5 0 0 1 3.4 11.5"/><path d="M12.8 2v2.8H10M3.2 14v-2.8H6"/>'),
    diff: svg('<path d="M5 2.5v7M1.5 6h7"/><path d="M8 13.5h6.5"/><path d="M11 2.5 14.5 6 11 9.5"/>'),
    heart: svg('<path d="M8 13.5S2 10 2 5.9A3.1 3.1 0 0 1 8 4.6a3.1 3.1 0 0 1 6 1.3C14 10 8 13.5 8 13.5z"/>'),
    // GitHub 마크 (Octicons mark-github, MIT) — 채운 모양이라 stroke 대신 fill
    github: '<svg class="ic" viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>',
    pull: svg('<path d="M8 13.5v-8M4.8 8.6 8 5.4l3.2 3.2"/><path d="M2.5 2.5h11"/>'),
  };

  // 에이전트 → 공식 로고 (ui/shared/logos.js)
  const t = k => (window.SL_I18N ? window.SL_I18N.t(k) : k);
  const AGENTS = {
    'claude-code': { label: 'Claude Code', logo: 'claudecode-color' },
    'codex': { label: 'Codex', logo: 'codex-color' },
    'gemini': { label: 'Gemini CLI', logo: 'geminicli-color' },
    'grok': { label: 'Grok Build', logo: 'grok' },
    'antigravity': { label: 'Antigravity', logo: 'antigravity-color' },
    'cursor': { label: 'Cursor', logo: 'cursor' },
    'windsurf': { label: 'Windsurf', logo: 'windsurf' },
    'claude-app': { label: () => t('Claude 앱'), logo: 'claude-color' },
    'other': { label: () => t('기타'), logo: null },
  };

  // 같은 로고를 여러 번 그려도 그라데이션 id가 겹치지 않게 매번 새 id를 붙여요.
  let seq = 0;
  function logo(name) {
    const svg = (window.SL_LOGO_SVG || {})[name];
    if (!svg) return I.folder;
    const n = ++seq;
    return svg.replace(/id="([^"]+)"/g, `id="$1-${n}"`).replace(/url\(#([^)]+)\)/g, `url(#$1-${n})`)
      .replace('<svg ', '<svg class="logo" aria-hidden="true" ');
  }

  /** size: '' (18px) | 'lg' (타일 안 22px) */
  function agentBadge(agent, size) {
    const a = AGENTS[agent] || AGENTS.other;
    return `<span class="agent ${size || ''}" title="${typeof a.label === 'function' ? a.label() : a.label}">${a.logo ? logo(a.logo) : I.folder}</span>`;
  }

  window.SL_ICONS = { I, AGENTS, agentBadge, logo };
})();
