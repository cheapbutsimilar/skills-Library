/* JS ↔ Rust 브리지. 모든 페이지는 window.skillsLib 하나만 써요.
   - 앱에서는 Tauri invoke/listen을 감싸요.
   - 브라우저 미리보기에서는 scripts/mock-api.js가 같은 모양의 객체(window.skillsLibMock)를 줘요. */
(function () {
  'use strict';
  const T = window.__TAURI__;
  if (!T || !T.core) {
    if (!window.skillsLibMock) console.warn('No Tauri and no mock API — open it with npm run preview');
    window.skillsLib = window.skillsLibMock;
    return;
  }
  const invoke = (cmd, args) => T.core.invoke(cmd, args || {});
  window.skillsLib = {
    isMock: false,
    call: invoke,
    init: () => invoke('init'),
    /** 이벤트: snapshot, settings, event{type: toast|focusSearch|drag|imported|update|hotkey} */
    on: (name, cb) => T.event.listen(name, e => cb(e.payload)),
    setSettings: patch => invoke('settings_set', { patch }),
  };
})();
