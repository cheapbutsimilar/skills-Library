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

  // ── 라이브러리 ──
  const lib = {
    'pdf-tools': {
      tags: ['문서'], favorite: true, updated: now - 26 * H,
      files: {
        'SKILL.md': md('pdf-tools', 'PDF에서 글자·표를 뽑고, 합치고, 나눠요. 사용자가 PDF 파일을 다루거나 양식을 채워 달라고 할 때 써요.', '# PDF 도구\n\n## 언제 쓰나요\n\n- PDF에서 텍스트나 표를 뽑을 때\n- 여러 PDF를 하나로 합칠 때\n\n## 방법\n\n1. `scripts/extract.py`로 텍스트를 뽑아요.\n2. 표는 `reference/tables.md`를 참고해요.\n'),
        'scripts/extract.py': 'import sys\nprint("extract", sys.argv[1:])\n',
        'reference/tables.md': '# 표 뽑기\n\npdfplumber를 써요.\n',
      },
    },
    'commit-message': {
      tags: ['git'], favorite: false, updated: now - 0.4 * H,
      files: { 'SKILL.md': md('commit-message', 'git diff를 읽고 Conventional Commits 형식의 한국어 커밋 메시지를 써요. 커밋 메시지를 써 달라고 하거나 커밋하기 전에 써요.', '# 커밋 메시지\n\n1. `git diff --staged`를 읽어요.\n2. 제목은 50자 안으로, 본문은 무엇을·왜.\n') },
    },
    'code-review-ko': {
      tags: ['git', '리뷰'], favorite: true, updated: now - 3 * 24 * H,
      files: { 'SKILL.md': md('code-review-ko', '변경된 코드를 읽고 버그·보안·가독성 순서로 한국어 리뷰를 남겨요. 코드 리뷰나 PR 검토를 부탁받으면 써요.', '# 코드 리뷰\n\n- 버그 먼저, 그다음 보안, 마지막으로 가독성.\n- 근거가 되는 줄 번호를 같이 적어요.\n'), 'checklist.md': '# 체크리스트\n\n- [ ] 경계값\n- [ ] 에러 처리\n' },
    },
    'api-docs': {
      tags: [], favorite: false, updated: now - 50 * 60e3,
      files: { 'SKILL.md': md('api-docs', 'API 문서 작성', '# API 문서\n\nOpenAPI 스펙에서 문서를 만들어요.\n') },
    },
    'brand-guidelines': {
      tags: ['디자인'], favorite: false, updated: now - 2 * H,
      files: { 'SKILL.md': md('brand-guidelines', '회사 브랜드 색·글꼴·말투를 지켜서 문서와 슬라이드를 만들어요. 발표 자료나 대외 문서를 만들 때 써요.', '# 브랜드 가이드\n\n- 키 컬러: #8B7CF6\n- 해요체, 짧은 문장\n') },
    },
    'sql-helper': {
      tags: ['데이터'], favorite: false, updated: now - 5 * 24 * H,
      files: { 'SKILL.md': md('sql-helper', '', '# SQL\n\n쿼리를 다듬어요.\n') },
    },
  };

  // ── 설정 ──
  let settings = {
    v: 1,
    theme: { mode: 'dark', preset: 'iris', bg: '', text: '', accent: '', font: 'pretendard', zoom: 100 },
    libraryDir: '',
    targets: [
      { id: 'claude-code', name: 'Claude Code', agent: 'claude-code', path: '~/.claude/skills', enabled: true, builtin: true },
      { id: 'codex', name: 'Codex', agent: 'codex', path: '~/.agents/skills', enabled: true, builtin: true },
      { id: 'codex-legacy', name: 'Codex (예전 위치)', agent: 'codex', path: '$CODEX_HOME/skills', enabled: false, builtin: true },
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

  const copies = { 'claude-code': {}, 'codex': {}, 'codex-legacy': {} };
  const man = { 'claude-code': {}, 'codex': {}, 'codex-legacy': {}, 'claude-app': {} };
  function put(t, slug, files) { copies[t][slug] = { files: clone(files) }; man[t][slug] = hash(files); }

  put('claude-code', 'pdf-tools', lib['pdf-tools'].files);
  put('codex', 'pdf-tools', lib['pdf-tools'].files);
  man['claude-app']['pdf-tools'] = hash(lib['pdf-tools'].files);
  put('claude-code', 'commit-message', lib['commit-message'].files);
  // Codex의 commit-message는 예전 버전 → 업데이트 있음
  put('codex', 'commit-message', Object.assign({}, lib['commit-message'].files, { 'SKILL.md': lib['commit-message'].files['SKILL.md'].replace('50자', '72자') }));
  // Claude Code에서 누가 고친 code-review-ko → 설치본이 바뀜
  put('claude-code', 'code-review-ko', lib['code-review-ko'].files);
  copies['claude-code']['code-review-ko'].files['local-notes.md'] = '팀 규칙 추가';
  put('codex', 'brand-guidelines', lib['brand-guidelines'].files);
  man['claude-app']['brand-guidelines'] = 'old';
  // 라이브러리에 없는 외부 스킬
  copies['claude-code']['frontend-design'] = { files: { 'SKILL.md': md('frontend-design', '눈에 띄는 프런트엔드 화면을 만들어요.', '# 프런트엔드') } };
  copies['codex']['webapp-testing'] = { files: { 'SKILL.md': md('webapp-testing', 'Playwright로 웹앱을 확인해요.', '# 테스트') } };
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
    if (!p.fm) { E('no-frontmatter', '맨 위에 ---로 감싼 머리말(name, description)이 필요해요'); return out; }
    if (!p.name) E('name-empty', 'name이 비어 있어요');
    else if (p.name.length > 64) E('name-long', `name은 64자까지예요 (지금 ${p.name.length}자)`);
    else if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(p.name)) E('name-chars', 'name은 영어 소문자·숫자·하이픈(-)만 쓸 수 있어요 (하이픈은 처음·끝·연속 불가)');
    if (/claude|anthropic/.test(p.name)) W('name-reserved', "name에 'claude'·'anthropic'이 들어가면 Claude 앱에 올릴 수 없어요");
    if (p.name && /^[a-z0-9]+(-[a-z0-9]+)*$/.test(p.name) && p.name !== dir) W('name-dir', `폴더 이름(${dir})과 name이 달라요 — 저장하면 폴더 이름을 name에 맞춰요`);
    if (!p.description) E('desc-empty', 'description이 비어 있어요 — 언제 쓰는 스킬인지 적어 주세요');
    else if (p.description.length > 1024) E('desc-long', `description은 1024자까지예요 (지금 ${p.description.length}자)`);
    if (/<\/?[a-zA-Z][^>]*>/.test(p.description)) E('desc-xml', 'description에 XML 태그(<…>)는 쓸 수 없어요');
    if (p.description && p.description.length < 20) W('desc-short', 'description이 짧아요 — 무엇을 하고 언제 쓰는지 적으면 에이전트가 더 잘 골라요');
    if (!p.body.trim()) W('body-empty', '본문이 비어 있어요');
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

  function snapshot() {
    const enabled = settings.targets.filter(t => t.enabled);
    const counts = { skills: 0, withErrors: 0, installed: 0, updates: 0, attention: 0, external: 0, targets: enabled.length };
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
    return { libraryDir: settings.libraryDir || '/home/me/.local/share/com.cheapbutsimilar.skillslibrary/library', skills, targets, counts, claudeAppUrl: 'https://claude.ai/settings/capabilities', at: Date.now() };
  }

  function changed() { const s = snapshot(); emit('snapshot', s); return s; }
  const need = slug => { if (!lib[slug]) throw '라이브러리에 그 스킬이 없어요'; return lib[slug]; };
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
      settings: clone(settings), snapshot: snapshot(), version: '0.1.0', platform: 'windows', release: true,
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
      if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name)) throw '이름은 영어 소문자·숫자·하이픈(-)으로 64자까지 쓸 수 있어요';
      if (lib[name]) throw `'${name}' 스킬이 이미 있어요`;
      lib[name] = { tags: [], favorite: false, updated: Date.now(), files: { 'SKILL.md': md(name, yamlStr(description), `# ${name}\n\n## 언제 쓰나요\n\n- 이 스킬이 필요한 상황을 적어 주세요.\n\n## 방법\n\n1. 첫 번째 단계\n2. 두 번째 단계\n`) } };
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
    file_read: ({ slug, path }) => { const f = need(slug).files[path]; if (f == null) throw '파일이 없어요'; return f; },
    file_write: ({ slug, path, content }) => {
      if (path.includes('..') || path.startsWith('/')) throw '파일 경로가 올바르지 않아요';
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
      lib[dir] = { tags: [], favorite: false, updated: Date.now(), files: clone(c.files) };
      man[target][dir] = hash(c.files);
      return { slug: dir, snapshot: changed() };
    },
    import_all_external: () => {
      const slugs = [];
      for (const t of settings.targets.filter(t => t.enabled)) for (const dir in copies[t.id]) if (!lib[dir]) { handlers.import_target({ target: t.id, dir }); slugs.push(dir); }
      return { slugs, errors: [], snapshot: changed() };
    },
    sync_all: () => { const done = pushUpdates(Object.keys(lib)); return { done: done.length, errors: [], snapshot: changed() }; },
    import_pick: () => {
      const slug = 'imported-skill' + (n++ ? '-' + n : '');
      lib[slug] = { tags: [], favorite: false, updated: Date.now(), files: { 'SKILL.md': md(slug, '가져온 스킬이에요. 미리보기에서는 가짜 스킬이 들어와요.', '# 가져온 스킬\n') } };
      return { slugs: [slug], errors: [], snapshot: changed() };
    },
    import_paths: () => handlers.import_pick(),
    export_zip: ({ slug }) => { man['claude-app'][slug] = hash(need(slug).files); changed(); return `/home/me/Downloads/${slug}.zip`; },
    target_add_project: ({ agent }) => {
      const add = (a, sub) => { const id = 't-' + (settings.targets.length + 1); settings.targets.push({ id, name: `my-app · ${a === 'codex' ? 'Codex' : 'Claude Code'}`, agent: a, path: `~/work/my-app/${sub}`, enabled: true, builtin: false }); copies[id] = {}; man[id] = {}; };
      if (agent !== 'codex') add('claude-code', '.claude/skills');
      if (agent !== 'claude-code') add('codex', '.agents/skills');
      emit('settings', clone(settings)); return clone(settings);
    },
    target_pick_path: () => clone(settings),
    target_reset_path: () => clone(settings),
    target_remove: ({ id }) => { settings.targets = settings.targets.filter(t => t.id !== id); return clone(settings); },
    library_change_dir: () => clone(settings),
    open_path: () => toast('미리보기에서는 폴더를 열지 않아요'),
    open_skill_dir: () => toast('미리보기에서는 폴더를 열지 않아요'),
    open_special: () => toast('미리보기에서는 폴더를 열지 않아요'),
    open_url: ({ url }) => { window.open(url, '_blank'); },
    win_minimize: () => null, win_toggle_maximize: () => null, win_close: () => toast('미리보기에서는 창을 닫지 않아요'),
    hotkey_suspend: () => null, hotkey_resume: () => null,
    update_check: () => null,
    update_install: () => { toast('미리보기에서는 설치하지 않아요'); },
  };

  window.skillsLibMock = {
    isMock: true,
    call: (cmd, args) => new Promise((resolve, reject) => {
      const h = handlers[cmd];
      if (!h) return reject(`mock에 없는 명령: ${cmd}`);
      setTimeout(() => { try { resolve(h(args || {})); } catch (e) { reject(e); } }, 30);
    }),
    init: () => Promise.resolve(handlers.init()),
    on: (name, cb) => { (listeners[name] = listeners[name] || []).push(cb); return Promise.resolve(() => {}); },
    setSettings: patch => window.skillsLibMock.call('settings_set', { patch }),
  };
})();
