/* 브라우저 미리보기용 가짜 브리지. window.skillsLib과 같은 모양(window.skillsLibMock)을 줘요.
   상태 계산(최신 / 업데이트 있음 / 설치본이 바뀜 …)은 Rust core와 같은 규칙을 따라요.
   devserver가 HTML에 이 파일을 끼워 넣어요. 실제 앱에서는 쓰지 않아요. */
(function () {
  'use strict';
  if (window.__TAURI__) return;

  const now = Date.now();
  const H = 3600e3;
  const listeners = {};
  const emit = (name, payload) => (listeners[name] || []).forEach(cb => setTimeout(() => cb(payload), 0));
  const clone = v => JSON.parse(JSON.stringify(v));

  const md = (name, desc, body) => `---\nname: ${name}\ndescription: ${desc}\n---\n\n${body}`;

  // 미리보기 언어: ?lang=en|ko, 없으면 브라우저 언어. 가짜 데이터도 그 언어로 만들어요.
  const qsL = new URLSearchParams(location.search).get('lang');
  let EN = qsL ? qsL === 'en' : !/^ko/i.test(navigator.language || '');
  const L = (ko, en) => (EN ? en : ko);

  // ── 라이브러리 ──
  const lib = {
    'pdf-tools': {
      tags: [L('문서', 'docs')], favorite: true, updated: now - 26 * H,
      files: {
        'SKILL.md': md('pdf-tools', L('PDF에서 글자·표를 뽑고, 합치고, 나눠요. 사용자가 PDF 파일을 다루거나 양식을 채워 달라고 할 때 써요.', 'Extract text and tables from PDFs, merge and split them. Use when the user works with PDF files or asks to fill in forms.'),
          L('# PDF 도구\n\n## 언제 쓰나요\n\n- PDF에서 텍스트나 표를 뽑을 때\n- 여러 PDF를 하나로 합칠 때\n\n## 방법\n\n1. `scripts/extract.py`로 텍스트를 뽑아요.\n2. 표는 `reference/tables.md`를 참고해요.\n',
            '# PDF tools\n\n## When to use\n\n- Extracting text or tables from a PDF\n- Merging several PDFs into one\n\n## Steps\n\n1. Run `scripts/extract.py` to pull the text.\n2. For tables, see `reference/tables.md`.\n')),
        'scripts/extract.py': 'import sys\nprint("extract", sys.argv[1:])\n',
        'reference/tables.md': L('# 표 뽑기\n\npdfplumber를 써요.\n', '# Tables\n\nUse pdfplumber.\n'),
      },
    },
    'commit-message': {
      tags: ['git'], favorite: false, updated: now - 0.4 * H,
      files: { 'SKILL.md': md('commit-message', L('git diff를 읽고 Conventional Commits 형식의 한국어 커밋 메시지를 써요. 커밋 메시지를 써 달라고 하거나 커밋하기 전에 써요.', 'Reads the git diff and writes a Conventional Commits message. Use when asked for a commit message or before committing.'),
        L('# 커밋 메시지\n\n1. `git diff --staged`를 읽어요.\n2. 제목은 50자 안으로, 본문은 무엇을·왜.\n', '# Commit message\n\n1. Read `git diff --staged`.\n2. Subject under 50 characters; body says what and why.\n')) },
    },
    'code-review-ko': {
      tags: ['git', L('리뷰', 'review')], favorite: true, updated: now - 3 * 24 * H,
      files: { 'SKILL.md': md('code-review-ko', L('변경된 코드를 읽고 버그·보안·가독성 순서로 한국어 리뷰를 남겨요. 코드 리뷰나 PR 검토를 부탁받으면 써요.', 'Reviews changed code for bugs, then security, then readability. Use when asked to review code or a pull request.'),
        L('# 코드 리뷰\n\n- 버그 먼저, 그다음 보안, 마지막으로 가독성.\n- 근거가 되는 줄 번호를 같이 적어요.\n', '# Code review\n\n- Bugs first, then security, then readability.\n- Cite the line numbers behind each point.\n')),
        'checklist.md': L('# 체크리스트\n\n- [ ] 경계값\n- [ ] 에러 처리\n', '# Checklist\n\n- [ ] Edge cases\n- [ ] Error handling\n') },
    },
    'api-docs': {
      tags: [], favorite: false, updated: now - 50 * 60e3,
      files: { 'SKILL.md': md('api-docs', L('API 문서 작성', 'API docs'), L('# API 문서\n\nOpenAPI 스펙에서 문서를 만들어요.\n', '# API docs\n\nGenerates docs from an OpenAPI spec.\n')) },
    },
    'brand-guidelines': {
      tags: [L('디자인', 'design')], favorite: false, updated: now - 2 * H,
      files: { 'SKILL.md': md('brand-guidelines', L('회사 브랜드 색·글꼴·말투를 지켜서 문서와 슬라이드를 만들어요. 발표 자료나 대외 문서를 만들 때 써요.', 'Applies the company colors, fonts and voice to docs and slides. Use for presentations and external documents.'),
        L('# 브랜드 가이드\n\n- 키 컬러: #8B7CF6\n- 해요체, 짧은 문장\n', '# Brand guidelines\n\n- Key color: #8B7CF6\n- Friendly, short sentences\n')) },
    },
    'sql-helper': {
      tags: [L('데이터', 'data')], favorite: false, updated: now - 5 * 24 * H,
      files: { 'SKILL.md': md('sql-helper', '', L('# SQL\n\n쿼리를 다듬어요.\n', '# SQL\n\nPolishes queries.\n')) },
    },
    'release-notes': {
      tags: ['git'], favorite: false, updated: now - 0.1 * H,
      files: { 'SKILL.md': md('release-notes', L('머지된 PR과 커밋을 모아 사용자용 릴리스 노트를 써요. 새 버전을 낼 때나 변경 내역을 정리해 달라고 할 때 써요.', 'Turns merged PRs and commits into user-facing release notes. Use when shipping a version or summarizing changes.'),
        L('# 릴리스 노트\n\n1. 지난 태그 이후 머지된 PR을 모아요.\n2. 새 기능 · 고친 점 · 바뀐 점으로 나눠요.\n3. 사용자 입장에서 한 줄씩 써요.\n', '# Release notes\n\n1. Collect PRs merged since the last tag.\n2. Group them into Added · Fixed · Changed.\n3. Write one user-facing line each.\n')) },
    },
  };

  const qs0 = new URLSearchParams(location.search);

  // ── 설정 ──
  let settings = {
    v: 3, language: qsL || 'system',
    theme: { mode: 'dark', preset: 'iris', bg: '', text: '', accent: '', font: 'pretendard', zoom: 100 },
    libraryDir: '',
    targets: [
      { id: 'claude-code', name: 'Claude Code', agent: 'claude-code', path: '~/.claude/skills', enabled: true, builtin: true },
      { id: 'codex', name: 'Codex', agent: 'codex', path: '~/.agents/skills', enabled: true, builtin: true },
      { id: 'codex-legacy', name: L('Codex (예전 위치)', 'Codex (legacy location)'), agent: 'codex', path: '$CODEX_HOME/skills', enabled: !qs0.get('nolegacy'), builtin: true },
      { id: 'gemini', name: 'Gemini CLI', agent: 'gemini', path: '~/.gemini/skills', enabled: true, builtin: true },
      { id: 'grok', name: 'Grok Build', agent: 'grok', path: '~/.grok/skills', enabled: !qs0.get('nogrok'), builtin: true },
      { id: 'antigravity', name: 'Antigravity', agent: 'antigravity', path: '~/.gemini/antigravity/skills', enabled: true, builtin: true },
      { id: 'cursor', name: 'Cursor', agent: 'cursor', path: '~/.cursor/skills', enabled: true, builtin: true },
      { id: 'windsurf', name: 'Windsurf', agent: 'windsurf', path: '~/.codeium/windsurf/skills', enabled: !qs0.get('nowindsurf'), builtin: true },
    ],
    autoSync: true, autostart: false, autoUpdate: true, closeToTray: true,
    hotkeyEnabled: true, hotkey: 'CommandOrControl+Alt+K', notifyExternal: true, sort: 'name', window: null,
  };
  const qs = new URLSearchParams(location.search);
  if (qs.get('mode')) settings.theme.mode = qs.get('mode');
  if (qs.get('preset')) settings.theme.preset = qs.get('preset');
  if (qs.get('empty')) for (const k of Object.keys(lib)) delete lib[k];

  const displayPath = p => p.replace('$CODEX_HOME', '~/.codex');

  // ── 해시·설치본 ──
  function hash(files) {
    const s = Object.keys(files).sort().map(k => k + '\0' + files[k]).join('\n');
    let h = 2166136261;
    for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
    return (h >>> 0).toString(16);
  }

  const TIDS = ['claude-code', 'codex', 'codex-legacy', 'gemini', 'grok', 'antigravity', 'cursor', 'windsurf'];
  const copies = Object.fromEntries(TIDS.map(id => [id, {}]));
  const man = Object.fromEntries([...TIDS, 'claude-app'].map(id => [id, {}]));
  function put(t, slug, files) { copies[t][slug] = { files: clone(files), updated: (lib[slug] || {}).updated || Date.now() }; man[t][slug] = hash(files); }

  put('claude-code', 'pdf-tools', lib['pdf-tools'].files);
  put('codex', 'pdf-tools', lib['pdf-tools'].files);
  man['claude-app']['pdf-tools'] = hash(lib['pdf-tools'].files);
  put('claude-code', 'commit-message', lib['commit-message'].files);
  // Codex의 commit-message는 예전 버전 → 업데이트 있음
  put('codex', 'commit-message', Object.assign({}, lib['commit-message'].files, { 'SKILL.md': lib['commit-message'].files['SKILL.md'].replace(L('50자', '50 characters'), L('72자', '72 characters')) }));
  // Claude Code에서 누가 고친 code-review-ko → 설치본이 바뀜
  put('claude-code', 'code-review-ko', lib['code-review-ko'].files);
  copies['claude-code']['code-review-ko'].files['local-notes.md'] = L('팀 규칙 추가\n- PR마다 테스트 결과를 붙여요.\n', 'Team rules\n- Attach test results to every PR.\n');
  copies['claude-code']['code-review-ko'].files['SKILL.md'] = lib['code-review-ko'].files['SKILL.md'].replace(L('- 근거가 되는 줄 번호를 같이 적어요.', '- Cite the line numbers behind each point.'), L('- 근거가 되는 줄 번호를 같이 적어요.\n- 팀 규칙(local-notes.md)을 먼저 확인해요.', '- Cite the line numbers behind each point.\n- Check the team rules in local-notes.md first.'));
  copies['claude-code']['code-review-ko'].updated = now - 1 * H;
  // 예전 Codex 위치: 같은 것(pdf-tools), 이름만 같은 다른 것(brand-guidelines), 거기에만 있는 것(figma)
  copies['codex-legacy']['pdf-tools'] = { files: clone(lib['pdf-tools'].files), updated: lib['pdf-tools'].updated };
  copies['codex-legacy']['brand-guidelines'] = { files: { 'SKILL.md': md('brand-guidelines', L('예전 브랜드 가이드예요. 색과 글꼴만 다뤄요.', 'The old brand guide. Covers only colors and fonts.'), L('# 브랜드 (예전)\n', '# Brand (old)\n')) }, updated: now - 40 * 24 * H };
  copies['codex-legacy']['figma'] = { files: { 'SKILL.md': md('figma', L('Figma 파일에서 디자인 정보를 읽어 코드로 옮겨요.', 'Reads design details from Figma files and turns them into code.'), '# Figma\n') }, updated: now - 9 * 24 * H };
  put('codex', 'brand-guidelines', lib['brand-guidelines'].files);
  put('gemini', 'pdf-tools', lib['pdf-tools'].files);
  put('gemini', 'code-review-ko', lib['code-review-ko'].files);
  put('grok', 'commit-message', lib['commit-message'].files);
  put('cursor', 'pdf-tools', lib['pdf-tools'].files);
  put('antigravity', 'commit-message', lib['commit-message'].files);
  man['claude-app']['brand-guidelines'] = 'old';
  // 라이브러리에 없는 외부 스킬
  copies['claude-code']['frontend-design'] = { updated: now - 2 * 24 * H, files: { 'SKILL.md': md('frontend-design', L('눈에 띄는 프런트엔드 화면을 만들어요.', 'Builds distinctive frontend screens.'), L('# 프런트엔드', '# Frontend')) } };
  copies['codex']['webapp-testing'] = { updated: now - 6 * 24 * H, files: { 'SKILL.md': md('webapp-testing', L('Playwright로 웹앱을 확인해요.', 'Checks web apps with Playwright.'), L('# 테스트', '# Testing')) } };
  if (qs0.get('demo')) {
    // 데모 GIF용: 설치·확인할 게 적은 깔끔한 상태
    for (const tid of TIDS) { for (const k of ['frontend-design', 'webapp-testing']) delete copies[tid][k]; }
    put('codex', 'commit-message', lib['commit-message'].files);
    copies['claude-code']['code-review-ko'] = { files: clone(lib['code-review-ko'].files), updated: lib['code-review-ko'].updated };
    man['claude-code']['code-review-ko'] = hash(lib['code-review-ko'].files);
  }
  if (qs.get('empty')) { for (const t in copies) copies[t] = {}; for (const t in man) man[t] = {}; }

  // ── 파싱·검사 (Rust core의 간단한 판) ──
  function parse(raw) {
    const m = /^﻿?---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/.exec(raw);
    if (!m) return { fm: false, name: '', description: '', body: raw };
    const get = k => {
      const r = new RegExp('^' + k + ':\\s*(.*)$', 'm').exec(m[1]);
      if (!r) return '';
      let v = r[1].trim();
      if (v.startsWith('"')) { try { v = JSON.parse(v); } catch (e) { /* 그대로 */ } }
      return v;
    };
    return { fm: true, name: get('name'), description: get('description'), body: m[2] };
  }
  function check(raw, dir) {
    const p = parse(raw), out = [];
    const E = (code, msg) => out.push({ level: 'error', code, msg }), W = (code, msg) => out.push({ level: 'warn', code, msg });
    if (!p.fm) { E('no-frontmatter', L('맨 위에 ---로 감싼 머리말(name, description)이 필요해요', "A frontmatter block wrapped in --- (name, description) is required at the top")); return out; }
    if (!p.name) E('name-empty', L('name이 비어 있어요', "name is empty"));
    else if (p.name.length > 64) E('name-long', L(`name은 64자까지예요 (지금 ${p.name.length}자)`, `name can be up to 64 characters (now ${p.name.length})`));
    else if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(p.name)) E('name-chars', L('name은 영어 소문자·숫자·하이픈(-)만 쓸 수 있어요 (하이픈은 처음·끝·연속 불가)', "name may only use lowercase letters, digits and hyphens (no leading, trailing or double hyphens)"));
    if (/claude|anthropic/.test(p.name)) W('name-reserved', L("name에 'claude'·'anthropic'이 들어가면 Claude 앱에 올릴 수 없어요", "Names containing 'claude' or 'anthropic' cannot be uploaded to the Claude app"));
    if (p.name && /^[a-z0-9]+(-[a-z0-9]+)*$/.test(p.name) && p.name !== dir) W('name-dir', L(`폴더 이름(${dir})과 name이 달라요 — 저장하면 폴더 이름을 name에 맞춰요`, `The folder name (${dir}) differs from name — saving renames the folder to match`));
    if (!p.description) E('desc-empty', L('description이 비어 있어요 — 언제 쓰는 스킬인지 적어 주세요', "description is empty — say when the skill should be used"));
    else if (p.description.length > 1024) E('desc-long', L(`description은 1024자까지예요 (지금 ${p.description.length}자)`, `description can be up to 1024 characters (now ${p.description.length})`));
    if (/<\/?[a-zA-Z][^>]*>/.test(p.description)) E('desc-xml', L('description에 XML 태그(<…>)는 쓸 수 없어요', "description cannot contain XML tags (<…>)"));
    if (p.description && p.description.length < 20) W('desc-short', L('description이 짧아요 — 무엇을 하고 언제 쓰는지 적으면 에이전트가 더 잘 골라요', "description is short — saying what it does and when to use it helps agents pick it"));
    if (!p.body.trim()) W('body-empty', L('본문이 비어 있어요', "The body is empty"));
    return out;
  }

  // ── 스냅샷 ──
  function status(lh, th, rec) {
    if (th == null) return 'notInstalled';
    if (th === lh) return 'synced';
    if (rec != null && th === rec) return 'libraryNewer';
    if (rec != null && rec === lh) return 'targetModified';
    if (rec != null) return 'conflict';
    return 'foreign';
  }
  const bytesOf = files => Object.values(files).reduce((a, s) => a + new Blob([s]).size, 0);

  const ATTN = ['targetModified', 'conflict', 'foreign'];
  const isOn = id => settings.targets.some(t => t.id === id && t.enabled);

  function snapshot() {
    const enabled = settings.targets.filter(t => t.enabled);
    const counts = { skills: 0, withErrors: 0, installed: 0, updates: 0, attention: 0, external: 0, targets: enabled.length, divergent: 0, legacySkills: 0 };
    const skills = Object.keys(lib).sort().map(slug => {
      const s = lib[slug];
      const lh = hash(s.files);
      const p = parse(s.files['SKILL.md'] || '');
      const issues = check(s.files['SKILL.md'] || '', slug);
      if (issues.some(i => i.level === 'error')) counts.withErrors++;
      const installs = {};
      for (const t of enabled) {
        const c = (copies[t.id] || {})[slug];
        const st = status(lh, c ? hash(c.files) : null, (man[t.id] || {})[slug]);
        if (st === 'synced') (man[t.id] = man[t.id] || {})[slug] = lh;
        if (st !== 'notInstalled') counts.installed++;
        if (st === 'libraryNewer') counts.updates++;
        if (['targetModified', 'conflict', 'foreign'].includes(st)) counts.attention++;
        installs[t.id] = st;
      }
      if (enabled.some(t => ATTN.includes(installs[t.id]))) counts.divergent++;
      const ar = man['claude-app'][slug];
      installs['claude-app'] = ar == null ? 'notInstalled' : ar === lh ? 'synced' : 'libraryNewer';
      return {
        slug, name: p.name || slug, description: p.description, hash: lh, files: Object.keys(s.files).length,
        bytes: bytesOf(s.files), updated: s.updated, issues, tags: s.tags, favorite: s.favorite, installs,
      };
    });
    counts.skills = skills.length;
    const targets = settings.targets.map(t => {
      const list = t.enabled ? Object.keys(copies[t.id] || {}).sort().map(dir => {
        const c = copies[t.id][dir];
        const p = parse(c.files['SKILL.md'] || '');
        const sk = skills.find(s => s.slug === dir);
        return { dir, name: p.name || dir, description: p.description, inLibrary: !!sk, status: sk ? sk.installs[t.id] : null, errors: 0, symlink: false };
      }) : [];
      if (t.enabled) counts.external += list.filter(k => !k.inLibrary).length;
      return {
        id: t.id, name: t.name, agent: t.agent, path: displayPath(t.path), fullPath: '/home/me/' + displayPath(t.path).replace('~/', ''),
        enabled: t.enabled, builtin: t.builtin, exists: Object.keys(copies[t.id] || {}).length > 0 || t.id !== 'codex-legacy', skills: list,
      };
    });
    if (isOn('codex-legacy') && isOn('codex')) counts.legacySkills = Object.keys(copies['codex-legacy']).length;
    return { libraryDir: settings.libraryDir || '/home/me/.local/share/com.cheapbutsimilar.skillslibrary/library', skills, targets, counts, claudeAppUrl: 'https://claude.ai/settings/capabilities', at: Date.now() };
  }

  // ── 차이 비교 (Rust core의 diff.rs와 같은 모양, 작은 파일용 LCS) ──
  function lineOps(a, b) {
    const n = a.length, m = b.length;
    const dp = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
    for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    const ops = [];
    let i = 0, j = 0;
    while (i < n || j < m) {
      if (i < n && j < m && a[i] === b[j]) { ops.push([' ', i, j]); i++; j++; }
      else if (j < m && (i >= n || dp[i][j + 1] >= dp[i + 1][j])) { ops.push(['+', i, j]); j++; }
      else { ops.push(['-', i, j]); i++; }
    }
    return ops;
  }
  function textHunks(at, bt) {
    const split = s => (s === '' ? [] : s.replace(/\n$/, '').split('\n'));
    const a = split(at), b = split(bt);
    const ops = lineOps(a, b);
    const changedIdx = ops.map((o, k) => (o[0] !== ' ' ? k : -1)).filter(k => k >= 0);
    const hunks = [];
    for (let x = 0; x < changedIdx.length; x++) {
      const start = Math.max(0, changedIdx[x] - 3);
      let end = changedIdx[x];
      while (x + 1 < changedIdx.length && changedIdx[x + 1] <= end + 7) end = changedIdx[++x];
      end = Math.min(ops.length - 1, end + 3);
      const lines = ops.slice(start, end + 1).map(([op, ai, bi]) => ({ op, a: op !== '+' ? ai + 1 : null, b: op !== '-' ? bi + 1 : null, text: op === '+' ? b[bi] : a[ai] }));
      hunks.push({ aStart: ops[start][1] + 1, bStart: ops[start][2] + 1, lines });
    }
    return { hunks, added: ops.filter(o => o[0] === '+').length, removed: ops.filter(o => o[0] === '-').length };
  }
  function dirDiff(fa, fb) {
    const out = { files: [], added: 0, removed: 0, modified: 0, same: 0 };
    for (const path of [...new Set([...Object.keys(fa), ...Object.keys(fb)])].sort()) {
      const inA = path in fa, inB = path in fb;
      const change = !inB ? 'removed' : !inA ? 'added' : fa[path] === fb[path] ? 'same' : 'modified';
      out[change]++;
      const h = change === 'same' ? { hunks: [], added: 0, removed: 0 } : textHunks(inA ? fa[path] : '', inB ? fb[path] : '');
      out.files.push(Object.assign({ path, change, text: true, aBytes: inA ? new Blob([fa[path]]).size : 0, bBytes: inB ? new Blob([fb[path]]).size : 0, truncated: false }, h));
    }
    out.files.sort((x, y) => (x.change === 'same') - (y.change === 'same') || (x.path !== 'SKILL.md') - (y.path !== 'SKILL.md') || x.path.localeCompare(y.path));
    return out;
  }

  function changed() { const s = snapshot(); emit('snapshot', s); return s; }
  const need = slug => { if (!lib[slug]) throw L('라이브러리에 그 스킬이 없어요', 'That skill is not in the library'); return lib[slug]; };
  const toast = msg => emit('event', { type: 'toast', msg });

  function merge(a, b) {
    for (const k in b) {
      if (b[k] && typeof b[k] === 'object' && !Array.isArray(b[k]) && a[k] && typeof a[k] === 'object') merge(a[k], b[k]);
      else a[k] = b[k];
    }
    return a;
  }

  function install(slug, tid) { need(slug); put(tid, slug, lib[slug].files); }
  function pushUpdates(slugs) {
    const done = [];
    const snap = snapshot();
    for (const s of snap.skills) {
      if (!slugs.includes(s.slug)) continue;
      for (const t of settings.targets.filter(t => t.enabled)) {
        if (s.installs[t.id] === 'libraryNewer') { install(s.slug, t.id); done.push(t.name); }
      }
    }
    return done;
  }
  function renameInstalls(old, nu) {
    for (const t in copies) {
      if (man[t] && man[t][old] && copies[t][old] && hash(copies[t][old].files) === man[t][old]) {
        delete copies[t][old]; delete man[t][old];
        put(t, nu, lib[nu].files);
      }
    }
  }
  function saveRaw(slug, raw) {
    const s = need(slug);
    s.files['SKILL.md'] = raw;
    s.updated = Date.now();
    const p = parse(raw);
    let nu = slug;
    if (p.name && p.name !== slug && /^[a-z0-9]+(-[a-z0-9]+)*$/.test(p.name) && !lib[p.name]) {
      lib[p.name] = s; delete lib[slug]; nu = p.name; renameInstalls(slug, nu);
    }
    const synced = settings.autoSync ? pushUpdates([nu]) : [];
    return { slug: nu, synced, errors: [], snapshot: changed() };
  }
  const yamlStr = v => /[:#"'\n]|^\s|\s$/.test(v) ? JSON.stringify(v) : v;

  let n = 0;
  const handlers = {
    init: () => ({
      settings: clone(settings), snapshot: snapshot(), version: '0.1.0', platform: qs.get('platform') || 'windows', release: true,
      update: qs.get('update') ? { version: '0.1.42', notes: '' } : null,
      dataDir: '/home/me/.local/share/com.cheapbutsimilar.skillslibrary',
      defaultLibrary: '/home/me/.local/share/com.cheapbutsimilar.skillslibrary/library', hotkeyError: null, firstRun: false,
      firstImported: qs.get('welcome') ? 4 : 0,
    }),
    settings_set: ({ patch }) => { merge(settings, clone(patch)); const s = clone(settings); emit('settings', s); changed(); return s; },
    refresh: () => changed(),
    skill_get: ({ slug }) => {
      const s = need(slug); const raw = s.files['SKILL.md'] || ''; const p = parse(raw);
      return {
        slug, dir: '/home/me/library/' + slug, raw, name: p.name, description: p.description, body: p.body,
        files: Object.keys(s.files).sort().map(path => ({ path, bytes: new Blob([s.files[path]]).size, text: true })), issues: check(raw, slug),
      };
    },
    skill_check: ({ slug, raw }) => check(raw, slug),
    skill_save: ({ slug, raw }) => saveRaw(slug, raw),
    skill_save_fields: ({ slug, name, description, body }) => {
      const raw = need(slug).files['SKILL.md'];
      const m = /^---\r?\n([\s\S]*?)\r?\n---/.exec(raw);
      const rest = m ? m[1].split('\n').filter(l => !/^(name|description):/.test(l)) : [];
      const fm = [`name: ${yamlStr(name.trim())}`, `description: ${yamlStr(description.trim())}`, ...rest].join('\n');
      return saveRaw(slug, `---\n${fm}\n---\n\n${body.replace(/^\n/, '')}`);
    },
    skill_create: ({ name, description }) => {
      if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name)) throw L('이름은 영어 소문자·숫자·하이픈(-)으로 64자까지 쓸 수 있어요', 'Names use lowercase letters, digits and hyphens (-), up to 64 characters');
      if (lib[name]) throw L(`'${name}' 스킬이 이미 있어요`, `A skill named '${name}' already exists`);
      lib[name] = { tags: [], favorite: false, updated: Date.now(), files: { 'SKILL.md': md(name, yamlStr(description), L(`# ${name}\n\n## 언제 쓰나요\n\n- 이 스킬이 필요한 상황을 적어 주세요.\n\n## 방법\n\n1. 첫 번째 단계\n2. 두 번째 단계\n`, `# ${name}\n\n## When to use\n\n- Describe when this skill is needed.\n\n## Steps\n\n1. First step\n2. Second step\n`)) } };
      return { slug: name, snapshot: changed() };
    },
    skill_duplicate: ({ slug }) => {
      const s = need(slug); let nu = slug + '-copy'; let i = 2;
      while (lib[nu]) nu = `${slug}-copy-${i++}`;
      lib[nu] = clone(s); lib[nu].favorite = false; lib[nu].updated = Date.now();
      lib[nu].files['SKILL.md'] = lib[nu].files['SKILL.md'].replace(/^name:.*$/m, 'name: ' + nu);
      return { slug: nu, snapshot: changed() };
    },
    skill_delete: ({ slug, uninstall }) => {
      need(slug);
      if (uninstall) for (const t in copies) if (copies[t][slug] && man[t][slug] === hash(copies[t][slug].files)) { delete copies[t][slug]; delete man[t][slug]; }
      delete lib[slug]; delete man['claude-app'][slug];
      return changed();
    },
    skill_meta: ({ slug, tags, favorite }) => {
      const s = need(slug);
      if (tags) s.tags = [...new Set(tags.map(t => t.trim()).filter(Boolean))].sort();
      if (favorite != null) s.favorite = favorite;
      return changed();
    },
    file_read: ({ slug, path }) => { const f = need(slug).files[path]; if (f == null) throw L('파일이 없어요', 'The file does not exist'); return f; },
    file_write: ({ slug, path, content }) => {
      if (path.includes('..') || path.startsWith('/')) throw L('파일 경로가 올바르지 않아요', 'The file path is not valid');
      need(slug).files[path] = content; lib[slug].updated = Date.now();
      const synced = settings.autoSync ? pushUpdates([slug]) : [];
      return { slug, synced, errors: [], snapshot: changed() };
    },
    file_delete: ({ slug, path }) => { delete need(slug).files[path]; return { slug, synced: settings.autoSync ? pushUpdates([slug]) : [], errors: [], snapshot: changed() }; },
    install: ({ slug, target }) => { install(slug, target); return changed(); },
    uninstall: ({ slug, target }) => { delete copies[target][slug]; delete man[target][slug]; return changed(); },
    pull: ({ slug, target }) => {
      const c = copies[target][slug]; need(slug).files = clone(c.files); lib[slug].updated = Date.now(); man[target][slug] = hash(c.files);
      const synced = settings.autoSync ? pushUpdates([slug]) : [];
      return { slug, synced, errors: [], snapshot: changed() };
    },
    import_target: ({ target, dir }) => {
      const c = copies[target][dir];
      lib[dir] = { tags: [], favorite: false, updated: c.updated || Date.now(), files: clone(c.files) };
      man[target][dir] = hash(c.files);
      return { slug: dir, snapshot: changed() };
    },
    import_all_external: () => {
      const slugs = [];
      for (const t of settings.targets.filter(t => t.enabled)) for (const dir in copies[t.id]) if (!lib[dir]) { handlers.import_target({ target: t.id, dir }); slugs.push(dir); }
      return { slugs, errors: [], snapshot: changed() };
    },
    cleanup_plan: () => {
      const snap = snapshot();
      const enabled = settings.targets.filter(t => t.enabled);
      const divergent = [];
      for (const sk of snap.skills) {
        if (!enabled.some(t => ATTN.includes(sk.installs[t.id]))) continue;
        const vs = [];
        const add = (files, updated, place) => {
          const h = hash(files);
          let v = vs.find(x => x.hash === h);
          if (!v) { v = { hash: h, files: Object.keys(files).length, bytes: bytesOf(files), updated, places: [], inLibrary: false }; vs.push(v); }
          v.updated = Math.max(v.updated, updated); v.places.push(place); if (place.id === 'library') v.inLibrary = true;
        };
        add(lib[sk.slug].files, lib[sk.slug].updated, { id: 'library', name: L('라이브러리', 'Library'), agent: null });
        for (const t of enabled) { const c = (copies[t.id] || {})[sk.slug]; if (c) add(c.files, c.updated || 0, { id: t.id, name: t.name, agent: t.agent }); }
        vs.sort((a, b) => b.updated - a.updated);
        divergent.push({ slug: sk.slug, description: sk.description, versions: vs, suggested: vs[0].hash });
      }
      let legacy = null;
      const from = copies['codex-legacy'], to = copies['codex'];
      if (isOn('codex-legacy') && isOn('codex') && Object.keys(from).length) {
        legacy = {
          fromName: L('Codex (예전 위치)', 'Codex (legacy location)'), fromPath: '~/.codex/skills', toName: 'Codex', toPath: '~/.agents/skills',
          skills: Object.keys(from).sort().map(dir => ({ dir, state: !to[dir] ? 'move' : hash(to[dir].files) === hash(from[dir].files) ? 'duplicate' : 'differs' })),
        };
      }
      return { divergent, legacy, external: snap.counts.external };
    },
    cleanup_apply: ({ choices }) => {
      const rep = { imported: 0, unified: 0, moved: 0, duplicates: 0, skipped: [], errors: [], legacyEmptied: false };
      const enabled = () => settings.targets.filter(t => t.enabled);
      if (choices.external) rep.imported = handlers.import_all_external().slugs.length;
      for (const [slug, want] of Object.entries(choices.picks || {})) {
        if (!want || !lib[slug]) continue;
        if (hash(lib[slug].files) !== want) {
          const src = enabled().map(t => (copies[t.id] || {})[slug]).find(c => c && hash(c.files) === want);
          if (!src) { rep.errors.push(L(`${slug}: 고른 버전을 찾지 못했어요 — 그 사이에 바뀌었어요. 다시 열어 주세요`, `${slug}: could not find the chosen version — it changed in the meantime. Please reopen`)); continue; }
          lib[slug].files = clone(src.files); lib[slug].updated = src.updated || Date.now();
        }
        for (const t of enabled()) if ((copies[t.id] || {})[slug]) put(t.id, slug, lib[slug].files);
        rep.unified++;
      }
      if (choices.legacy && isOn('codex-legacy') && isOn('codex')) {
        const from = copies['codex-legacy'], to = copies['codex'];
        for (const dir of Object.keys(from).sort()) {
          if (!lib[dir]) { rep.skipped.push(L(`${dir}: 라이브러리에 없어서 그대로 뒀어요`, `${dir}: left as is because it is not in the library`)); continue; }
          const lh = hash(lib[dir].files);
          if (hash(from[dir].files) !== lh || (to[dir] && hash(to[dir].files) !== lh)) { rep.skipped.push(L(`${dir}: 내용이 서로 달라서 그대로 뒀어요 — 버전을 먼저 골라 주세요`, `${dir}: left as is because the copies differ — pick a version first`)); continue; }
          if (!to[dir]) { put('codex', dir, lib[dir].files); rep.moved++; } else rep.duplicates++;
          delete from[dir]; delete man['codex-legacy'][dir];
        }
        rep.legacyEmptied = !Object.keys(from).length;
        if (rep.legacyEmptied) { settings.targets.find(t => t.id === 'codex-legacy').enabled = false; emit('settings', clone(settings)); }
      }
      return { report: rep, settings: clone(settings), snapshot: changed() };
    },
    set_lang: ({ lang }) => { EN = lang === 'en'; return changed(); },
    diff: ({ slug, target }) => {
      const s = need(slug);
      const c = (copies[target] || {})[slug];
      if (!c) throw L('설치 위치에 그 스킬이 없어요', 'That skill is not in this location');
      const t = settings.targets.find(x => x.id === target);
      return { slug, target, targetName: t ? t.name : target, libraryPath: '/home/me/library/' + slug, installedPath: '/home/me/' + displayPath(t.path).replace('~/', '') + '/' + slug, diff: dirDiff(s.files, c.files) };
    },
    install_all: ({ slug }) => {
      const snap = snapshot();
      const sk = snap.skills.find(x => x.slug === slug);
      if (!sk) throw L(L('라이브러리에 그 스킬이 없어요', 'That skill is not in the library'), 'That skill is not in the library');
      const done = [];
      for (const t of settings.targets.filter(x => x.enabled)) {
        const st = sk.installs[t.id];
        if (st === 'notInstalled' || st === 'libraryNewer') { install(slug, t.id); done.push(t.name); }
      }
      return { done, errors: [], snapshot: changed() };
    },
    import_github: ({ url }) => new Promise((resolve, reject) => setTimeout(() => {
      const m = /^(?:https?:\/\/)?(?:www\.)?(?:github\.com\/)?([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:\/(?:tree|blob)\/[^/]+\/?(.*))?\/?$/.exec((url || '').trim());
      if (!m) return reject(L('github.com 주소만 가져올 수 있어요', 'Only github.com URLs are supported'));
      const sub = (m[3] || '').replace(/\/SKILL\.md$/i, '').split('/').filter(Boolean);
      const names = sub.length ? [sub[sub.length - 1].toLowerCase()] : ['pdf', 'docx', 'xlsx'];
      const slugs = [];
      for (const base of names) {
        let slug = base; let i = 2;
        while (lib[slug]) slug = `${base}-${i++}`;
        lib[slug] = { tags: ['github'], favorite: false, updated: Date.now(), files: {
          'SKILL.md': md(slug, L(`${m[1]}/${m[2]}에서 가져온 스킬이에요. 미리보기에서는 가짜 내용이 들어와요.`, `Imported from ${m[1]}/${m[2]}. The preview fills in placeholder content.`), `# ${slug}\n`),
          'LICENSE.txt': 'MIT',
        } };
        slugs.push(slug);
      }
      resolve({ slugs, errors: [], snapshot: changed() });
    }, 900)),
    sync_all: () => { const done = pushUpdates(Object.keys(lib)); return { done: done.length, errors: [], snapshot: changed() }; },
    import_pick: () => {
      const slug = 'imported-skill' + (n++ ? '-' + n : '');
      lib[slug] = { tags: [], favorite: false, updated: Date.now(), files: { 'SKILL.md': md(slug, L('가져온 스킬이에요. 미리보기에서는 가짜 스킬이 들어와요.', 'An imported skill. The preview adds a placeholder skill.'), L('# 가져온 스킬\n', '# Imported skill\n')) } };
      return { slugs: [slug], errors: [], snapshot: changed() };
    },
    import_paths: () => handlers.import_pick(),
    share_upload: ({ slugs }) => new Promise(resolve => setTimeout(() => {
      const picked = (slugs && slugs.length ? slugs : Object.keys(lib));
      const chars = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
      let code = '';
      for (let i = 0; i < 7; i++) code += chars[Math.floor(Math.random() * chars.length)];
      resolve({ code, url: `https://skills-library-share.sakugarchive.workers.dev/bundles/${code}`, expiresInDays: 30, bytes: 3600 * picked.length, count: picked.length });
    }, 700)),
    import_share: ({ input }) => new Promise((resolve, reject) => setTimeout(() => {
      const v = (input || '').trim();
      if (!/^[A-Za-z0-9]{7}$/.test(v) && !/skills-library-share/i.test(v)) return reject(L('공유 코드가 아니에요 — 7자리 코드나 공유 링크를 붙여 넣어 주세요', 'That is not a share code — paste a 7-character code or a share link'));
      const slug = 'shared-skill' + (n++ ? '-' + n : '');
      lib[slug] = { tags: ['shared'], favorite: false, updated: Date.now(), files: { 'SKILL.md': md(slug, L('공유 코드로 받은 스킬이에요. 미리보기에서는 가짜 내용이 들어와요.', 'A skill received by share code. The preview fills in placeholder content.'), `# ${slug}\n`) } };
      resolve({ slugs: [slug], errors: [], snapshot: changed() });
    }, 700)),
    export_zip: ({ slug }) => { man['claude-app'][slug] = hash(need(slug).files); changed(); return `/home/me/Downloads/${slug}.zip`; },
    target_add_project: ({ agent }) => {
      const sub = { 'claude-code': '.claude/skills', codex: '.agents/skills', gemini: '.gemini/skills', grok: '.grok/skills', antigravity: '.agent/skills', cursor: '.cursor/skills', windsurf: '.windsurf/skills' };
      const label = { 'claude-code': 'Claude Code', codex: 'Codex', gemini: 'Gemini', grok: 'Grok', antigravity: 'Antigravity', cursor: 'Cursor', windsurf: 'Windsurf' };
      for (const a of agent.split(',')) {
        if (!sub[a]) continue;
        const id = 't-' + (settings.targets.length + 1);
        settings.targets.push({ id, name: `my-app · ${label[a]}`, agent: a, path: `~/work/my-app/${sub[a]}`, enabled: true, builtin: false });
        copies[id] = {}; man[id] = {};
      }
      emit('settings', clone(settings)); return clone(settings);
    },
    target_pick_path: () => clone(settings),
    target_reset_path: () => clone(settings),
    target_remove: ({ id }) => { settings.targets = settings.targets.filter(t => t.id !== id); return clone(settings); },
    library_change_dir: () => clone(settings),
    open_path: () => toast(L('미리보기에서는 폴더를 열지 않아요', 'The preview does not open folders')),
    open_skill_dir: () => toast(L('미리보기에서는 폴더를 열지 않아요', 'The preview does not open folders')),
    open_special: () => toast(L('미리보기에서는 폴더를 열지 않아요', 'The preview does not open folders')),
    open_url: ({ url }) => { window.open(url, '_blank'); },
    win_minimize: () => null, win_toggle_maximize: () => null, win_close: () => toast(L('미리보기에서는 창을 닫지 않아요', 'The preview does not close the window')),
    hotkey_suspend: () => null, hotkey_resume: () => null,
    update_check: () => null,
    update_install: () => { toast(L('미리보기에서는 설치하지 않아요', 'The preview does not install updates')); },
  };

  window.skillsLibMock = {
    isMock: true,
    call: (cmd, args) => new Promise((resolve, reject) => {
      const h = handlers[cmd];
      if (!h) return reject(`mock: unknown command ${cmd}`);
      setTimeout(() => { try { resolve(h(args || {})); } catch (e) { reject(e); } }, 30);
    }),
    init: () => Promise.resolve(handlers.init()),
    on: (name, cb) => { (listeners[name] = listeners[name] || []).push(cb); return Promise.resolve(() => {}); },
    setSettings: patch => window.skillsLibMock.call('settings_set', { patch }),
  };
})();
