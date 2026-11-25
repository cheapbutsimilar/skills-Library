/* 화면 글자 영어(en)·한국어(ko).
   - 원문(한국어)이 곧 키예요: t('설치했어요') → 영어면 'Installed'. 번역은 ui/shared/i18n-en.js에 있어요.
   - {이름} 자리에 값을 넣어요: t('업데이트 {n}개', { n: 3 })
   - 언어 설정: system(시스템 언어 따라가기) | en | ko
   - scripts/check-ui.js가 모든 t('…')에 영어 번역이 있는지 검사해요. */
(function () {
  'use strict';

  let lang = 'ko';

  function detect() {
    const list = (navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || 'en']);
    return /^ko\b/i.test(list[0] || '') ? 'ko' : 'en';
  }

  function resolve(setting) {
    return setting === 'ko' || setting === 'en' ? setting : detect();
  }

  function set(l) {
    lang = l === 'ko' ? 'ko' : 'en';
    document.documentElement.lang = lang;
  }

  function fill(s, vars) {
    if (!vars) return s;
    return s.replace(/\{(\w+)\}/g, (m, k) => (vars[k] != null ? String(vars[k]) : m));
  }

  function t(key, vars) {
    const EN = window.SL_I18N_EN || {};
    const s = lang === 'en' ? (EN[key] != null ? EN[key] : key) : key;
    return fill(s, vars);
  }

  window.SL_I18N = { t, set, resolve, detect, get lang() { return lang; } };
})();
