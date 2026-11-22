/* README·검토용 스크린샷. 미리보기 서버를 띄우고 모든 페이지를 찍어요.
   npm run screenshots            → docs/screenshots/*.png
   BASE=http://127.0.0.1:5199 node scripts/preview.js   (이미 띄운 서버 사용) */
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const { chromium } = require('playwright');

const ROOT = path.join(__dirname, '..');
const OUT = process.env.OUT || path.join(ROOT, 'docs', 'screenshots');
const BASE = process.env.BASE || 'http://127.0.0.1:5199';
const SCALE = Number(process.env.SCALE || 2);

async function waitUp(url, ms = 120000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    try { const r = await fetch(url); if (r.ok) return; } catch (e) { /* 아직 */ }
    await new Promise(r => setTimeout(r, 400));
  }
  throw new Error('미리보기 서버가 뜨지 않았어요');
}

(async () => {
  let server = null;
  if (!process.env.BASE) {
    server = spawn('cargo', ['run', '-q', '-p', 'skillslib-devserver'], { cwd: ROOT, stdio: 'inherit' });
  }
  try {
    await waitUp(BASE + '/dashboard/index.html');
    fs.mkdirSync(OUT, { recursive: true });
    const browser = await chromium.launch();
    const errors = [];
    const shots = [
      ['overview', 'dark', s => s.goto('overview')],
      ['library', 'dark', s => s.goto('library')],
      ['skill-install', 'dark', s => s.openSkill('commit-message')],
      ['skill-edit', 'dark', s => s.openSkill('api-docs', 'edit')],
      ['skill-files', 'dark', s => s.openSkill('pdf-tools', 'files')],
      ['targets', 'dark', s => s.goto('targets')],
      ['look', 'dark', s => s.goto('look')],
      ['alerts', 'dark', s => s.goto('alerts')],
      ['general', 'dark', s => s.goto('general')],
      ['overview-light', 'light', s => s.goto('overview')],
      ['library-light', 'light', s => s.goto('library')],
    ];
    for (const [name, mode, act] of shots) {
      const page = await browser.newPage({ viewport: { width: 1060, height: 720 }, deviceScaleFactor: SCALE });
      page.on('pageerror', e => errors.push(`${name}: ${e.message}`));
      page.on('console', m => { if (m.type() === 'error') errors.push(`${name}: ${m.text()}`); });
      await page.goto(`${BASE}/dashboard/index.html?mode=${mode}`);
      await page.waitForFunction(() => window.__SL && window.__SL.S.snap);
      await page.evaluate(`(${act.toString()})(window.__SL)`);
      await page.waitForTimeout(350);
      await page.screenshot({ path: path.join(OUT, `${name}.png`) });
      await page.close();
    }
    await browser.close();
    if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
    console.log(`스크린샷 ${shots.length}장 → ${OUT}`);
  } finally {
    if (server) server.kill();
  }
})().catch(e => { console.error(e); process.exit(1); });
