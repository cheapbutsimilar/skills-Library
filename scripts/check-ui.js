/* UI 정적 검사: JS 문법, 인라인 <style> 금지(HANDOFF §5.2). CI에서 돌려요. */
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const ROOT = path.join(__dirname, '..');
let bad = 0;
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
    try { new vm.Script(src, { filename: p }); } catch (e) { console.error(`문법 오류: ${p}\n  ${e.message}`); bad++; }
  }
  if (p.endsWith('.html') && /<style[\s>]/i.test(src)) {
    console.error(`인라인 <style> 금지: ${p} — CSP 해시가 생겨 style="" 속성이 모두 막혀요`); bad++;
  }
}
walk(path.join(ROOT, 'ui'));
check(path.join(ROOT, 'scripts', 'mock-api.js'));
if (bad) process.exit(1);
console.log('UI 검사 통과');
