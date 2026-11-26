/* 포맷·공통 도우미 */
(function () {
  'use strict';

  const esc = s => String(s == null ? '' : s)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#39;');

  function bytes(n) {
    if (!n) return '0 B';
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(n < 10240 ? 1 : 0)} KB`;
    return `${(n / 1024 / 1024).toFixed(1)} MB`;
  }

  function compact(n) {
    if (n < 1000) return String(n);
    if (n < 1e6) return (n / 1e3).toFixed(n < 1e4 ? 2 : 1).replace(/\.0+$/, '') + 'K';
    return (n / 1e6).toFixed(2).replace(/\.0+$/, '') + 'M';
  }

  const t = (k, v) => (window.SL_I18N ? window.SL_I18N.t(k, v) : k);

  /** "방금", "3분 전", "2시간 13분 전", "3일 전" (영어: "just now", "3m ago" …) */
  function ago(ms, now) {
    if (!ms) return '—';
    now = now || Date.now();
    const s = Math.max(0, Math.round((now - ms) / 1000));
    if (s < 45) return t('방금');
    const m = Math.round(s / 60);
    if (m < 60) return t('{m}분 전', { m });
    const h = Math.floor(m / 60);
    if (h < 24) return m % 60 && h < 6 ? t('{h}시간 {m}분 전', { h, m: m % 60 }) : t('{h}시간 전', { h });
    const d = Math.floor(h / 24);
    if (d < 30) return t('{d}일 전', { d });
    const dt = new Date(ms);
    return `${dt.getFullYear()}.${dt.getMonth() + 1}.${dt.getDate()}`;
  }

  function debounce(fn, ms) {
    let t = null;
    const f = (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); };
    f.cancel = () => clearTimeout(t);
    return f;
  }

  /** 키 이벤트 → Tauri 단축키 문자열 ("CommandOrControl+Alt+K").
      macOS에서는 ⌘(Command)가 CommandOrControl이고, ⌃(Control)는 따로 "Control"로 적어요. */
  function accelFromEvent(e, platform) {
    const mods = [];
    if (platform === 'macos') {
      if (e.metaKey) mods.push('CommandOrControl');
      if (e.ctrlKey) mods.push('Control');
    } else if (e.ctrlKey || e.metaKey) mods.push('CommandOrControl');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    let key = null;
    const c = e.code || '';
    if (/^Key[A-Z]$/.test(c)) key = c.slice(3);
    else if (/^Digit[0-9]$/.test(c)) key = c.slice(5);
    else if (/^F([1-9]|1[0-2])$/.test(c)) key = c;
    else if (c === 'Space') key = 'Space';
    if (!key || !mods.length) return null;
    return [...mods, key].join('+');
  }

  const MAC_KEYS = { CommandOrControl: '⌘', CmdOrCtrl: '⌘', Command: '⌘', Cmd: '⌘', Super: '⌘', Control: '⌃', Ctrl: '⌃', Alt: '⌥', Option: '⌥', Shift: '⇧' };
  const PC_KEYS = { CommandOrControl: 'Ctrl', CmdOrCtrl: 'Ctrl', Control: 'Ctrl', Command: 'Win', Cmd: 'Win', Super: 'Win', Option: 'Alt' };
  const MAC_ORDER = ['⌃', '⌥', '⇧', '⌘'];

  /** 보여줄 때는 짧게: "Ctrl+Alt+K", macOS는 기호를 붙여서 "⌥⌘K" (Apple 순서 ⌃⌥⇧⌘) */
  function accelLabel(a, platform) {
    if (!a) return '';
    const parts = a.split('+');
    if (platform !== 'macos') return parts.map(p => PC_KEYS[p] || p).join('+');
    const mods = parts.slice(0, -1).map(p => MAC_KEYS[p] || p).sort((x, y) => MAC_ORDER.indexOf(x) - MAC_ORDER.indexOf(y));
    return mods.join('') + parts[parts.length - 1];
  }

  /** 창 안 단축키 표시: Ctrl+S / ⌘S */
  function modKey(key, platform) {
    return platform === 'macos' ? '⌘' + key : 'Ctrl+' + key;
  }

  window.SL_UTIL = { esc, bytes, compact, ago, debounce, accelFromEvent, accelLabel, modKey };
})();
