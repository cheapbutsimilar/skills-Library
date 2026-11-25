/* 에이전트 공식 로고를 ui/shared/logos.js로 만들어요.
   출처: @lobehub/icons-static-svg (MIT) — https://github.com/lobehub/lobe-icons
   로고 자체의 상표권은 각 회사에 있어요. 어느 에이전트인지 알려주는 용도로만 써요.

     node scripts/make-logos.js <icons-static-svg 패키지 폴더>
   (패키지는 `npm pack @lobehub/icons-static-svg`로 받아서 풀면 돼요) */
const fs = require('fs');
const path = require('path');

const src = process.argv[2];
if (!src) { console.error('패키지 폴더를 알려 주세요'); process.exit(1); }
const pkg = JSON.parse(fs.readFileSync(path.join(src, 'package.json'), 'utf8'));

const NAMES = ['claude-color', 'claudecode-color', 'codex-color', 'gemini-color', 'geminicli-color', 'grok', 'antigravity-color', 'cursor', 'windsurf'];
const out = {};
for (const n of NAMES) {
  let s = fs.readFileSync(path.join(src, 'icons', n + '.svg'), 'utf8').trim();
  s = s.replace(/<title>.*?<\/title>/, '')
    .replace(/ style="[^"]*"/, '')
    .replace(/ (height|width)="1em"/g, '');
  out[n] = s;
}

const js = `/* 자동 생성 파일 — scripts/make-logos.js로 다시 만들어요. 직접 고치지 마세요.
   에이전트 로고: @lobehub/icons-static-svg ${pkg.version} (MIT, https://github.com/lobehub/lobe-icons)
   로고의 상표권은 Anthropic, OpenAI, Google, xAI, Anysphere, Cognition에 있어요. 어느 에이전트인지 표시하는 용도로만 써요. */
(function () {
  'use strict';
  window.SL_LOGO_SVG = ${JSON.stringify(out, null, 2)};
})();
`;
fs.writeFileSync(path.join(__dirname, '..', 'ui', 'shared', 'logos.js'), js);
console.log('ui/shared/logos.js', NAMES.length, '개');
