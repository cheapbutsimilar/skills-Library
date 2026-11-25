/* README 데모 GIF (docs/screenshots/demo.gif, 25fps).
   미리보기 서버의 실제 화면을 프레임마다 찍어서 만들어요:
   라이브러리에서 스킬 고르기 → 켜진 에이전트 전부에 한 번에 설치 → 고치고 저장하면 모든 설치본에 동기화.

     npm run demo                                   (미리보기 서버를 띄워서 찍어요, ffmpeg 필요)
     BASE=http://127.0.0.1:5199 node scripts/demo-gif.js
     LANG_UI=ko OUT=docs/screenshots/demo-ko.gif node scripts/demo-gif.js */
const { spawn, execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');
const { chromium } = require('playwright');

const ROOT = path.join(__dirname, '..');
const BASE = process.env.BASE || 'http://127.0.0.1:5199';
const OUT = process.env.OUT || path.join(ROOT, 'docs', 'screenshots', 'demo.gif');
const LANG = process.env.LANG_UI || 'en';
const FPS = 25;
const W = 1060, H = 720;
const GIF_W = Number(process.env.GIF_W || 960);

async function waitUp(url, ms = 120000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    try { const r = await fetch(url); if (r.ok) return; } catch (e) { /* 아직 */ }
    await new Promise(r => setTimeout(r, 400));
  }
  throw new Error('미리보기 서버가 뜨지 않았어요');
}

const ease = x => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);

(async () => {
  let server = null;
  if (!process.env.BASE) server = spawn('cargo', ['run', '-q', '-p', 'skillslib-devserver'], { cwd: ROOT, stdio: 'inherit' });
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'sl-demo-'));
  let n = 0;
  try {
    await waitUp(BASE + '/dashboard/index.html');
    const browser = await chromium.launch();
    const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: 1, locale: LANG === 'ko' ? 'ko-KR' : 'en-US' });
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.goto(`${BASE}/dashboard/index.html?mode=dark&lang=${LANG}&demo=1&nolegacy=1`);
    await page.waitForFunction(() => window.__SL && window.__SL.S.snap && window.__SL.S.lang);
    // 화면에 보이는 커서 (스크린샷에는 실제 마우스가 안 찍혀요)
    await page.evaluate(() => {
      const c = document.createElement('div');
      c.id = 'demo-cursor';
      c.innerHTML = '<svg width="22" height="22" viewBox="0 0 22 22"><path d="M3 2l15 9-6.5 1.6L8.2 19z" fill="#fff" stroke="#111" stroke-width="1.4" stroke-linejoin="round"/></svg><i></i>';
      Object.assign(c.style, { position: 'fixed', left: '0', top: '0', zIndex: 9999, pointerEvents: 'none', transform: 'translate(530px, 420px)' });
      const ring = c.querySelector('i');
      Object.assign(ring.style, { position: 'absolute', left: '-11px', top: '-11px', width: '26px', height: '26px', borderRadius: '50%', border: '2px solid #8B7CF6', opacity: '0', transform: 'scale(.4)' });
      document.body.appendChild(c);
      window.__cur = { x: 530, y: 420 };
      window.__setCur = (x, y, ring) => {
        window.__cur = { x, y };
        c.style.transform = `translate(${x}px, ${y}px)`;
        const r = c.querySelector('i');
        r.style.opacity = ring ? String(1 - ring) : '0';
        r.style.transform = `scale(${0.4 + (ring || 0) * 1.2})`;
      };
    });

    const frame = async (count = 1) => {
      const file = path.join(dir, `f${String(n).padStart(5, '0')}.png`);
      await page.screenshot({ path: file });
      n++;
      for (let i = 1; i < count; i++) {
        fs.copyFileSync(file, path.join(dir, `f${String(n).padStart(5, '0')}.png`));
        n++;
      }
    };
    const hold = secs => frame(Math.round(secs * FPS));
    const center = async sel => {
      const el = await page.waitForSelector(sel, { state: 'visible' });
      await el.scrollIntoViewIfNeeded();
      const b = await el.boundingBox();
      return { x: b.x + b.width / 2, y: b.y + b.height / 2 };
    };
    const moveTo = async (sel, secs = 0.6) => {
      const to = await center(sel);
      const from = await page.evaluate(() => window.__cur);
      const steps = Math.round(secs * FPS);
      for (let i = 1; i <= steps; i++) {
        const k = ease(i / steps);
        const x = from.x + (to.x - from.x) * k, y = from.y + (to.y - from.y) * k;
        await page.mouse.move(x, y);
        await page.evaluate(([x, y]) => window.__setCur(x, y), [x, y]);
        await frame();
      }
      return to;
    };
    const click = async (sel, after = 0.5) => {
      const p = await moveTo(sel);
      for (const r of [0.15, 0.4]) { await page.evaluate(([x, y, r]) => window.__setCur(x, y, r), [p.x, p.y, r]); await frame(); }
      await page.mouse.click(p.x, p.y);
      await page.waitForTimeout(120);
      for (const r of [0.65, 0.9]) { await page.evaluate(([x, y, r]) => window.__setCur(x, y, r), [p.x, p.y, r]); await frame(); }
      await page.evaluate(([x, y]) => window.__setCur(x, y), [p.x, p.y]);
      await hold(after);
    };
    const type = async text => {
      for (const ch of text) { await page.keyboard.type(ch); await frame(2); }
    };

    // 1. 라이브러리
    await page.evaluate(() => window.__SL.goto('library'));
    await page.waitForTimeout(200);
    await hold(1.0);
    // 2. 스킬 고르기
    await click('.skill[data-arg="release-notes"]', 0.9);
    // 3. 켜진 에이전트 전부에 한 번에 설치
    await moveTo('[data-target-row="cursor"] .tag', 0.5);
    await hold(0.4);
    await click('#install-all', 1.6);
    // 4. 고치고 저장 → 설치된 곳 전부 동기화
    await click('[data-seg="tab"][data-v="edit"]', 0.5);
    await click('#ed-body', 0.2);
    await page.evaluate(() => { const e = document.getElementById('ed-body'); e.setSelectionRange(e.value.length, e.value.length); e.scrollTop = e.scrollHeight; });
    await type(LANG === 'ko' ? '4. 사용자에게 고마움을 전해요.\n' : '4. Thank contributors by name.\n');
    await hold(0.3);
    await click('#edit-save', 1.6);
    // 5. 설치 탭: 모든 에이전트가 최신
    await click('[data-seg="tab"][data-v="install"]', 0.6);
    await moveTo('[data-target-row="windsurf"] .tag', 0.7);
    await hold(1.6);
    await browser.close();
    if (errors.length) throw new Error(errors.join('\n'));

    // ffmpeg: 프레임 → 팔레트 → GIF (바뀐 부분만 다시 그려서 작게)
    fs.mkdirSync(path.dirname(OUT), { recursive: true });
    const pal = path.join(dir, 'palette.png');
    const scale = `scale=${GIF_W}:-1:flags=lanczos`;
    execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(FPS), '-i', path.join(dir, 'f%05d.png'), '-vf', `${scale},palettegen=max_colors=192:stats_mode=diff`, pal]);
    execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(FPS), '-i', path.join(dir, 'f%05d.png'), '-i', pal,
      '-lavfi', `${scale}[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle`, '-r', String(FPS), OUT]);
    const kb = Math.round(fs.statSync(OUT).size / 1024);
    console.log(`${OUT} — ${n} frames, ${(n / FPS).toFixed(1)}s @ ${FPS}fps, ${kb} KB`);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
    if (server) server.kill();
  }
})().catch(e => { console.error(e); process.exit(1); });
