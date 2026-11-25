/* UI 정적 검사. CI에서 돌려요.
   - JS 문법
   - 인라인 <style> 금지 (HANDOFF §5.2)
   - 번역: 모든 t('…')에 영어 번역(ui/shared/i18n-en.js)이 있는지, t() 밖에 한국어 글자가 남지 않았는지 */
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const ROOT = path.join(__dirname, '..');
let bad = 0;
const jsFiles = [];
function walk(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else check(p);
  }
}
function check(p) {
  const src = fs.readFileSync(p, 'utf8');
  if (p.endsWith('.js')) {
    jsFiles.push(p);
    try { new vm.Script(src, { filename: p }); } catch (e) { console.error(`문법 오류: ${p}\n  ${e.message}`); bad++; }
  }
  if (p.endsWith('.html') && /<style[\s>]/i.test(src)) {
    console.error(`인라인 <style> 금지: ${p} — CSP 해시가 생겨 style="" 속성이 모두 막혀요`); bad++;
  }
}
walk(path.join(ROOT, 'ui'));
check(path.join(ROOT, 'scripts', 'mock-api.js'));

// ── 번역 ──
const sandbox = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(ROOT, 'ui', 'shared', 'i18n-en.js'), 'utf8'), sandbox);
const EN = sandbox.window.SL_I18N_EN || {};
const HANGUL = /[가-힣]/;
// 화면에 그대로 보여주는 고유 이름(언어 고르기의 '한국어', 데이터 파일 등)은 예외예요.
const SKIP = new Set(['logos.js', 'i18n-en.js', 'i18n.js', 'theme.js']);
const ALLOW_LINE = /'한국어'/;
const keyRe = /\bt\('((?:[^'\\]|\\.)*)'/g;
let keys = 0;
for (const p of jsFiles) {
  if (SKIP.has(path.basename(p)) || !p.includes(path.join('ui', ''))) continue;
  const src = fs.readFileSync(p, 'utf8');
  for (const m of src.matchAll(keyRe)) {
    keys++;
    const k = m[1].replace(/\\'/g, "'");
    if (EN[k] == null) { console.error(`번역 없음 (${path.relative(ROOT, p)}): ${k}`); bad++; }
  }
  // 주석과 t('…')를 지운 뒤에도 한국어가 남으면 번역을 빠뜨린 거예요.
  const stripped = src.replace(/\/\*[\s\S]*?\*\//g, m => m.replace(/[^\n]/g, ' ')).replace(keyRe, "t('')");
  stripped.split('\n').forEach((line, i) => {
    const code = line.replace(/(^|[^:'"`])\/\/.*$/, '$1');
    if (HANGUL.test(code) && !ALLOW_LINE.test(code)) {
      console.error(`번역되지 않은 글자 (${path.relative(ROOT, p)}:${i + 1}): ${code.trim().slice(0, 120)}`); bad++;
    }
  });
}
if (bad) process.exit(1);
console.log(`UI 검사 통과 (번역 키 ${keys}개 확인)`);
