/* 테마 엔진.
   사용자가 고르는 건 배경·글자·강조색 세 가지뿐이에요. 나머지 변수는 이 셋을 섞어서 계산해요.
   그래서 어떤 색을 골라도 화면 구조가 무너지지 않아요. */
(function () {
  'use strict';

  const PRESETS = {
    iris:   { name: '아이리스', dark: { bg: '#26262B', text: '#F3F3F5', accent: '#8B7CF6' }, light: { bg: '#F6F6F9', text: '#1C1C22', accent: '#5B47E0' } },
    ember:  { name: '노을',     dark: { bg: '#2B2B2B', text: '#F3F3F3', accent: '#F05C1A' }, light: { bg: '#FAF8F6', text: '#1E1B18', accent: '#C9480E' } },
    forest: { name: '숲',       dark: { bg: '#222825', text: '#EEF3EF', accent: '#3FB97A' }, light: { bg: '#F4F8F5', text: '#16201A', accent: '#1A7A4A' } },
    ocean:  { name: '바다',     dark: { bg: '#21262F', text: '#EEF2F7', accent: '#4A9CF5' }, light: { bg: '#F3F6FA', text: '#15202C', accent: '#1C64C4' } },
    ink:    { name: '먹',       dark: { bg: '#1D1D1D', text: '#F0F0F0', accent: '#E6E6E6' }, light: { bg: '#FFFFFF', text: '#111111', accent: '#242424' } },
    sand:   { name: '모래',     dark: { bg: '#2A2723', text: '#F4EFE8', accent: '#E0A93B' }, light: { bg: '#FBF7EF', text: '#231E16', accent: '#9A6A0E' } },
  };

  const FONTS = {
    pretendard: { name: 'Pretendard', css: '"Pretendard Variable", Pretendard, "Segoe UI Variable Text", "Segoe UI", "Malgun Gothic", system-ui, sans-serif' },
    segoe:      { name: 'Segoe UI',   css: '"Segoe UI Variable Text", "Segoe UI", "Malgun Gothic", system-ui, sans-serif' },
    malgun:     { name: '맑은 고딕',  css: '"Malgun Gothic", "Apple SD Gothic Neo", system-ui, sans-serif' },
    noto:       { name: 'Noto Sans KR', css: '"Noto Sans KR", "Noto Sans CJK KR", "Malgun Gothic", system-ui, sans-serif' },
    system:     { name: '시스템 기본', css: 'system-ui, -apple-system, sans-serif' },
  };

  const ZOOMS = [90, 100, 110, 125];

  function hex(c) {
    const h = c.replace('#', '');
    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
  }
  function toHex(rgb) {
    return '#' + rgb.map(v => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('').toUpperCase();
  }
  function mix(a, b, t) {
    const x = hex(a), y = hex(b);
    return toHex(x.map((v, i) => v + (y[i] - v) * t));
  }
  function rgba(c, a) {
    const [r, g, b] = hex(c);
    return `rgba(${r}, ${g}, ${b}, ${a})`;
  }
  function lum(c) {
    const f = v => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
    const [r, g, b] = hex(c);
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
  }
  /** WCAG 대비 */
  function contrast(a, b) {
    const x = lum(a), y = lum(b);
    return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
  }
  const isHex = c => typeof c === 'string' && /^#[0-9a-fA-F]{6}$/.test(c);

  function isDarkMode(theme) {
    if (theme.mode === 'dark') return true;
    if (theme.mode === 'light') return false;
    return !window.matchMedia || window.matchMedia('(prefers-color-scheme: dark)').matches;
  }

  /** 테마 설정 → 실제 세 가지 색 */
  function base(theme) {
    const p = PRESETS[theme.preset] || PRESETS.iris;
    const dark = isDarkMode(theme);
    const b = dark ? p.dark : p.light;
    return {
      dark,
      bg: isHex(theme.bg) ? theme.bg : b.bg,
      text: isHex(theme.text) ? theme.text : b.text,
      accent: isHex(theme.accent) ? theme.accent : b.accent,
    };
  }

  function derive({ dark, bg, text, accent }) {
    const onAccent = contrast(accent, '#FFFFFF') >= contrast(accent, '#141414') ? '#FFFFFF' : '#141414';
    const v = {
      '--bg': bg,
      '--bg-rgb': hex(bg).join(', '),
      '--bg-sunken': dark ? mix(bg, '#000000', 0.16) : mix(bg, text, 0.045),
      '--bg-raised': dark ? mix(bg, text, 0.05) : mix(bg, '#FFFFFF', 0.75),
      '--bg-hover': mix(bg, text, dark ? 0.08 : 0.06),
      '--bg-input': dark ? mix(bg, '#000000', 0.12) : mix(bg, '#FFFFFF', 0.85),
      '--line': rgba(text, dark ? 0.08 : 0.10),
      '--line-strong': rgba(text, dark ? 0.14 : 0.18),
      '--text': text,
      '--text-2': mix(text, bg, 0.30),
      '--text-3': mix(text, bg, 0.48),
      '--accent': accent,
      '--accent-hover': mix(accent, dark ? '#FFFFFF' : '#000000', 0.10),
      '--accent-press': mix(accent, dark ? '#000000' : '#000000', 0.10),
      '--accent-soft': rgba(accent, dark ? 0.14 : 0.10),
      '--accent-line': rgba(accent, 0.45),
      '--on-accent': onAccent,
      '--ok': dark ? '#3DD68C' : '#12844A',
      '--warn': dark ? '#F5B83D' : '#9A6100',
      '--crit': dark ? '#FF453A' : '#D21A0E',
      '--warn-soft': dark ? 'rgba(245, 184, 61, 0.14)' : 'rgba(154, 97, 0, 0.10)',
      '--crit-soft': dark ? 'rgba(255, 69, 58, 0.14)' : 'rgba(210, 26, 14, 0.09)',
      '--shadow': dark ? '0 12px 32px rgba(0, 0, 0, 0.45)' : '0 12px 32px rgba(20, 20, 40, 0.12)',
      '--tip-bg': dark ? mix(bg, text, 0.14) : mix(text, bg, 0.08),
      '--scroll': rgba(text, dark ? 0.16 : 0.20),
    };
    return v;
  }

  function apply(theme, root) {
    theme = theme || {};
    root = root || document.documentElement;
    const b = base(theme);
    const vars = derive(b);
    for (const k in vars) root.style.setProperty(k, vars[k]);
    root.style.setProperty('--font', (FONTS[theme.font] || FONTS.pretendard).css);
    root.style.colorScheme = b.dark ? 'dark' : 'light';
    root.dataset.mode = b.dark ? 'dark' : 'light';
    const z = ZOOMS.includes(theme.zoom) ? theme.zoom : 100;
    document.body && (document.body.style.zoom = String(z / 100));
    return b;
  }

  /** 시스템 모드를 따라가요 */
  function watch(getTheme) {
    if (!window.matchMedia) return;
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => apply(getTheme()));
  }

  window.SL_THEME = { PRESETS, FONTS, ZOOMS, apply, watch, base, contrast, mix, isHex };
})();
