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

  /** "방금", "3분 전", "2시간 13분 전", "3일 전" */
  function ago(ms, now) {
    if (!ms) return '—';
    now = now || Date.now();
    const s = Math.max(0, Math.round((now - ms) / 1000));
    if (s < 45) return '방금';
    const m = Math.round(s / 60);
    if (m < 60) return `${m}분 전`;
    const h = Math.floor(m / 60);
    if (h < 24) return m % 60 && h < 6 ? `${h}시간 ${m % 60}분 전` : `${h}시간 전`;
    const d = Math.floor(h / 24);
    if (d < 30) return `${d}일 전`;
    const dt = new Date(ms);
    return `${dt.getFullYear()}.${dt.getMonth() + 1}.${dt.getDate()}`;
  }

  function debounce(fn, ms) {
    let t = null;
    const f = (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); };
    f.cancel = () => clearTimeout(t);
    return f;
  }

  /** 키 이벤트 → Tauri 단축키 문자열 ("CommandOrControl+Alt+K") */
  function accelFromEvent(e) {
    const mods = [];
    if (e.ctrlKey || e.metaKey) mods.push('CommandOrControl');
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

  /** 보여줄 때는 짧게: "Ctrl+Alt+K" (macOS는 ⌘) */
  function accelLabel(a, platform) {
    if (!a) return '';
    const mac = platform === 'macos';
    return a.split('+').map(p => p === 'CommandOrControl' ? (mac ? '⌘' : 'Ctrl') : p === 'Alt' && mac ? '⌥' : p).join('+');
  }

  window.SL_UTIL = { esc, bytes, compact, ago, debounce, accelFromEvent, accelLabel };
})();
