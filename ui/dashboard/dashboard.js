/* 대시보드. 프레임워크 없이 문자열 템플릿으로 그리고, 바뀌면 페이지 전체를 다시 그려요.
   다시 그릴 때 스크롤 위치·포커스·커서 위치를 유지해요. */
(function () {
  'use strict';

  const api = window.skillsLib;
  const { esc, bytes, ago, debounce, accelFromEvent, accelLabel } = window.SL_UTIL;
  const { I, AGENTS, agentBadge } = window.SL_ICONS;
  const TH = window.SL_THEME;
  const $app = document.getElementById('app');
  const $modal = document.getElementById('modal-root');
  const $toast = document.getElementById('toast');
  const $drop = document.getElementById('drop');

  const S = {
    settings: null, snap: null, version: '', platform: '', release: false, update: null,
    dataDir: '', defaultLibrary: '', hotkeyError: null, firstImported: 0,
    page: 'overview', slug: null, tab: 'install', detail: null,
    draft: null, issues: null, file: null,
    q: '', tag: null, filter: 'all',
    creating: null, rec: false, modal: null, tagInput: '',
    lastKey: '',
  };

  // ───────────────────────── 공통 조각 ─────────────────────────
  const sw = (attrs, on) => `<label class="switch"><input type="checkbox" ${attrs} ${on ? 'checked' : ''}/><span></span></label>`;
  const row = (label, sub, control) => `<div class="row"><div class="lb">${label}${sub ? `<small>${sub}</small>` : ''}</div><div class="ctl">${control}</div></div>`;
  const seg = (name, options, value) => `<div class="seg">${options.map(([v, l]) =>
    `<button data-seg="${name}" data-v="${esc(v)}" class="${String(value) === String(v) ? 'active' : ''}">${l}</button>`).join('')}</div>`;
  const btn = (act, label, { cls = '', icon = '', arg = '', arg2 = '', title = '', disabled = false, id = '' } = {}) =>
    `<button class="btn ${cls}" data-act="${act}" ${arg !== '' ? `data-arg="${esc(arg)}"` : ''} ${arg2 !== '' ? `data-arg2="${esc(arg2)}"` : ''} ${title ? `title="${esc(title)}"` : ''} ${id ? `id="${id}"` : ''} ${disabled ? 'disabled' : ''}>${icon ? I[icon] : ''}${label ? `<span>${label}</span>` : ''}</button>`;
  const issueLine = i => `<div class="${i.level === 'error' ? 'err' : 'warn'}">${i.level === 'error' ? I.alert : I.info}<span>${esc(i.msg)}</span></div>`;

  const ST = {
    notInstalled: { label: '설치 안 됨', cls: 'muted' },
    synced: { label: '최신', cls: 'ok' },
    libraryNewer: { label: '업데이트 있음', cls: 'attn' },
    targetModified: { label: '설치본이 바뀜', cls: 'warn' },
    conflict: { label: '양쪽 다 바뀜', cls: 'crit' },
    foreign: { label: '이름 같은 다른 스킬', cls: 'warn' },
  };
  const ST_APP = {
    notInstalled: { label: '내보내지 않음', cls: 'muted' },
    synced: { label: '내보냄', cls: 'ok' },
    libraryNewer: { label: '내보낸 뒤 바뀜', cls: 'attn' },
  };
  const stTag = (st, app) => {
    const d = (app ? ST_APP : ST)[st] || ST.notInstalled;
    return `<span class="tag ${d.cls}">${d.label}</span>`;
  };
  const isAttn = st => st === 'targetModified' || st === 'conflict' || st === 'foreign';

  const enabledTargets = () => (S.snap ? S.snap.targets.filter(t => t.enabled) : []);
  const skillBy = slug => S.snap && S.snap.skills.find(s => s.slug === slug);
  const errCount = s => s.issues.filter(i => i.level === 'error').length;
  const warnCount = s => s.issues.filter(i => i.level === 'warn').length;

  function toast(msg, bad) {
    $toast.textContent = msg;
    $toast.classList.toggle('bad', !!bad);
    $toast.classList.add('show');
    clearTimeout(toast.t);
    toast.t = setTimeout(() => $toast.classList.remove('show'), bad ? 4200 : 2400);
  }

  async function run(fn) {
    try {
      return await fn();
    } catch (e) {
      toast(typeof e === 'string' ? e : (e && e.message) || String(e), true);
      return undefined;
    }
  }
  const call = (cmd, args) => api.call(cmd, args);
  const take = r => { if (r && r.snapshot) S.snap = r.snapshot; return r; };

  // ───────────────────────── 타이틀바·사이드바 ─────────────────────────
  function titlebar() {
    const c = S.snap ? S.snap.counts : null;
    let state = '';
    if (c) {
      if (c.updates) state = `<span class="state attn">업데이트 ${c.updates}개 대기</span>`;
      else if (c.attention) state = `<span class="state attn">확인 필요 ${c.attention}개</span>`;
      else state = `<span class="state" data-tauri-drag-region>모두 최신 · ${ago(S.snap.at)}</span>`;
    }
    const up = S.update ? `<button class="badge-up" data-act="goto" data-arg="general">v${esc(S.update.version)} 업데이트</button>` : '';
    return `<header class="titlebar" data-tauri-drag-region>
      <span class="mark" data-tauri-drag-region></span>
      <span class="app" data-tauri-drag-region>Skills Library</span>
      <span class="ver" data-tauri-drag-region>v${esc(S.version)}</span>
      <span class="drag" data-tauri-drag-region></span>
      ${state}${up}
      <button class="tb-btn" data-act="goto" data-arg="general" title="설정">${I.gear}</button>
      <button class="tb-btn" data-act="refresh" title="다시 검사">${I.refresh}</button>
      <span class="tb-sep"></span>
      <button class="tb-btn" data-act="winMin" title="최소화">${I.min}</button>
      <button class="tb-btn" data-act="winMax" title="최대화">${I.max}</button>
      <button class="tb-btn close" data-act="winClose" title="닫기">${I.close}</button>
    </header>`;
  }

  function side() {
    const c = S.snap ? S.snap.counts : {};
    const pg = S.page === 'skill' ? 'library' : S.page;
    const nav = (id, icon, label, count, hot) =>
      `<button class="nav ${pg === id ? 'active' : ''}" data-act="goto" data-arg="${id}">${I[icon]}<span class="label">${label}</span>${count ? `<span class="count ${hot ? 'hot' : ''}">${count}</span>` : ''}</button>`;
    const todo = (c.updates || 0) + (c.attention || 0);
    return `<nav class="side">
      ${nav('overview', 'overview', '개요', todo, todo > 0)}
      ${nav('library', 'library', '라이브러리', c.skills)}
      ${nav('targets', 'targets', '설치 위치', c.external, c.external > 0)}
      ${nav('look', 'look', '모양')}
      ${nav('alerts', 'bell', '알림·단축키')}
      ${nav('general', 'gear', '일반')}
      <div class="side-gap"></div>
      <div class="side-foot">
        <div class="mini"><span>자동 동기화</span>${sw('data-set="autoSync" aria-label="자동 동기화"', S.settings.autoSync)}</div>
        <div class="hint">
          ${S.settings.hotkeyEnabled ? `<div><span>어디서든 검색</span><kbd>${esc(accelLabel(S.settings.hotkey, S.platform))}</kbd></div>` : '<div><span>검색</span><kbd>/</kbd></div>'}
          <div><span>새 스킬</span><kbd>Ctrl+N</kbd></div>
        </div>
      </div>
    </nav>`;
  }

  // ───────────────────────── 개요 ─────────────────────────
  function todoItems() {
    const items = [];
    for (const s of S.snap.skills) {
      for (const t of enabledTargets()) {
        const st = s.installs[t.id];
        if (st === 'libraryNewer' || isAttn(st)) items.push({ kind: 'sync', s, t, st });
      }
      if (s.installs['claude-app'] === 'libraryNewer') items.push({ kind: 'app', s, st: 'libraryNewer' });
    }
    for (const t of enabledTargets()) {
      for (const k of t.skills) if (!k.inLibrary) items.push({ kind: 'external', t, k });
    }
    for (const s of S.snap.skills) if (errCount(s)) items.push({ kind: 'error', s });
    return items;
  }

  function todoRow(it) {
    if (it.kind === 'sync') {
      const acts = it.st === 'libraryNewer'
        ? btn('install', '업데이트', { cls: 'sm primary', arg: it.s.slug, arg2: it.t.id })
        : btn('pull', '가져오기', { cls: 'sm', arg: it.s.slug, arg2: it.t.id, title: '설치본 내용을 라이브러리로 가져와요' }) +
          btn('overwrite', '덮어쓰기', { cls: 'sm', arg: it.s.slug, arg2: it.t.id, title: '라이브러리 내용으로 덮어써요 (설치본은 휴지통에 백업)' });
      return `<tr class="click" data-act="open" data-arg="${esc(it.s.slug)}"><td>${agentBadge(it.t.agent)}</td>
        <td class="nm"><span class="mono">${esc(it.s.slug)}</span><small>${esc(it.t.name)} · ${esc(it.t.path)}</small></td>
        <td>${stTag(it.st)}</td><td><div class="acts">${acts}</div></td></tr>`;
    }
    if (it.kind === 'app') {
      return `<tr class="click" data-act="open" data-arg="${esc(it.s.slug)}"><td>${agentBadge('claude-app')}</td>
        <td class="nm"><span class="mono">${esc(it.s.slug)}</span><small>Claude 앱 · 다시 내보내서 올려 주세요</small></td>
        <td>${stTag(it.st, true)}</td><td><div class="acts">${btn('exportZip', 'ZIP 내보내기', { cls: 'sm', arg: it.s.slug })}</div></td></tr>`;
    }
    if (it.kind === 'external') {
      return `<tr><td>${agentBadge(it.t.agent)}</td>
        <td class="nm"><span class="mono">${esc(it.k.dir)}</span><small>${esc(it.t.name)}에만 있어요${it.k.description ? ' · ' + esc(it.k.description) : ''}</small></td>
        <td><span class="tag">라이브러리에 없음</span></td><td><div class="acts">${btn('importTarget', '라이브러리로 가져오기', { cls: 'sm', arg: it.t.id, arg2: it.k.dir })}</div></td></tr>`;
    }
    return `<tr class="click" data-act="openEdit" data-arg="${esc(it.s.slug)}"><td>${I.alert}</td>
      <td class="nm"><span class="mono">${esc(it.s.slug)}</span><small>${esc(it.s.issues.find(i => i.level === 'error').msg)}</small></td>
      <td><span class="tag crit">오류 ${errCount(it.s)}</span></td><td><div class="acts">${btn('openEdit', '고치기', { cls: 'sm', arg: it.s.slug })}</div></td></tr>`;
  }

  function pageOverview() {
    const c = S.snap.counts;
    const items = todoItems();
    const welcome = S.firstImported
      ? `<div class="card"><div class="tip-line">${I.check}<span>설치 위치에 있던 스킬 <b>${S.firstImported}개</b>를 라이브러리로 가져왔어요. 이제 여기서 고치면 설치된 곳에도 바로 반영돼요.</span></div></div><div class="spacer"></div>`
      : '';
    const ext = c.external ? btn('importAll', `밖에 있는 스킬 ${c.external}개 가져오기`, { icon: 'download' }) : '';
    return `<div class="page">
      <div class="page-head"><div class="t"><h1>개요</h1><p>라이브러리와 설치 위치의 상태를 한눈에 봐요.</p></div>
        <div class="actions">${ext}${c.updates ? btn('syncAll', `업데이트 ${c.updates}개 모두 적용`, { cls: 'primary', icon: 'sync' }) : ''}</div></div>
      ${welcome}
      <div class="grid4">
        <div class="card kpi"><span class="k">스킬</span><span class="v">${c.skills}</span><span class="s">${c.withErrors ? `오류 있는 스킬 ${c.withErrors}개` : '모두 규칙에 맞아요'}</span></div>
        <div class="card kpi"><span class="k">설치</span><span class="v">${c.installed}</span><span class="s">설치 위치 ${c.targets}곳에</span></div>
        <div class="card kpi"><span class="k">업데이트</span><span class="v ${c.updates ? 'attn' : ''}">${c.updates}</span><span class="s">${c.updates ? '라이브러리가 더 새로워요' : '모두 최신이에요'}</span></div>
        <div class="card kpi"><span class="k">확인 필요</span><span class="v ${c.attention ? 'warn' : ''}">${c.attention}</span><span class="s">${c.attention ? '설치 위치에서 바뀐 스킬' : '따로 바뀐 설치본 없음'}</span></div>
      </div>
      <h2>할 일 <span class="sub">${items.length ? `${items.length}개` : ''}</span></h2>
      <div class="card flush">${items.length ? `<table class="tbl"><colgroup><col class="c-agent"/><col/><col class="c-status"/><col class="c-act"/></colgroup>
        <thead><tr><th></th><th>스킬</th><th>상태</th><th class="r">동작</th></tr></thead><tbody>${items.map(todoRow).join('')}</tbody></table>`
        : `<div class="empty"><b>할 일이 없어요</b>라이브러리와 설치 위치가 모두 같아요.</div>`}</div>
      <h2>설치 위치</h2>
      <div class="card flush"><table class="tbl"><colgroup><col class="c-agent"/><col/><col class="c-num"/><col class="c-status"/></colgroup>
        <thead><tr><th></th><th>이름</th><th class="r">스킬</th><th>상태</th></tr></thead><tbody>
        ${S.snap.targets.map(t => `<tr class="click" data-act="goto" data-arg="targets"><td>${agentBadge(t.agent)}</td>
          <td class="nm">${esc(t.name)}<small class="mono">${esc(t.path)}</small></td>
          <td class="r">${t.enabled ? t.skills.length : '—'}</td>
          <td>${!t.enabled ? '<span class="status">꺼짐</span>' : !t.exists ? '<span class="status">폴더 없음 (설치할 때 만들어요)</span>' : '<span class="status ok">연결됨</span>'}</td></tr>`).join('')}
        <tr class="click" data-act="goto" data-arg="targets"><td>${agentBadge('claude-app')}</td><td class="nm">Claude 앱<small>claude.ai · 데스크톱 — ZIP으로 올려요</small></td>
          <td class="r">${S.snap.skills.filter(s => s.installs['claude-app'] !== 'notInstalled').length}</td><td><span class="status">내보내기</span></td></tr>
        </tbody></table></div>
    </div>`;
  }

  // ───────────────────────── 라이브러리 ─────────────────────────
  function filtered() {
    const q = S.q.trim().toLowerCase();
    let list = S.snap.skills.slice();
    if (q) list = list.filter(s => (s.slug + ' ' + s.name + ' ' + s.description + ' ' + s.tags.join(' ')).toLowerCase().includes(q));
    if (S.tag) list = list.filter(s => s.tags.includes(S.tag));
    const ids = enabledTargets().map(t => t.id);
    const anyInst = s => ids.some(id => s.installs[id] && s.installs[id] !== 'notInstalled');
    if (S.filter === 'installed') list = list.filter(anyInst);
    if (S.filter === 'none') list = list.filter(s => !anyInst(s));
    if (S.filter === 'updates') list = list.filter(s => ids.some(id => s.installs[id] === 'libraryNewer' || isAttn(s.installs[id])));
    if (S.filter === 'issues') list = list.filter(s => s.issues.length);
    const by = S.settings.sort === 'updated' ? (a, b) => b.updated - a.updated : (a, b) => a.slug.localeCompare(b.slug);
    list.sort((a, b) => (b.favorite - a.favorite) || by(a, b));
    return list;
  }

  function skillCard(s) {
    const pips = enabledTargets().map(t => {
      const st = s.installs[t.id];
      const cls = st === 'libraryNewer' ? 'on attn' : isAttn(st) ? 'on warn' : st && st !== 'notInstalled' ? 'on' : '';
      return `<span class="pip ${cls}" title="${esc(t.name)}: ${(ST[st] || ST.notInstalled).label}">${agentBadge(t.agent)}</span>`;
    }).join('');
    const e = errCount(s), w = warnCount(s);
    return `<button class="skill" data-act="open" data-arg="${esc(s.slug)}">
      <div class="top"><span class="nm">${esc(s.slug)}</span>${s.favorite ? `<span class="fav">${I.starOn}</span>` : ''}</div>
      <div class="desc ${s.description ? '' : 'none'}">${esc(s.description || '설명이 없어요')}</div>
      <div class="foot">${pips}<span class="gap"></span>
        ${s.tags.slice(0, 2).map(t => `<span class="tag">${esc(t)}</span>`).join('')}
        ${e ? `<span class="tag crit">오류 ${e}</span>` : w ? `<span class="tag warn">경고 ${w}</span>` : ''}
        <span class="tag muted">${ago(s.updated)}</span></div>
    </button>`;
  }

  function createCard() {
    const c = S.creating;
    const valid = /^[a-z0-9]+(-[a-z0-9]+)*$/.test(c.name) && c.name.length <= 64;
    const hint = !c.name ? '영어 소문자·숫자·하이픈(-)으로 써요. 예: commit-message'
      : valid ? `폴더 이름도 <span class="mono">${esc(c.name)}</span>이 돼요`
        : '<span class="muted">영어 소문자·숫자·하이픈(-)만 쓸 수 있어요 (하이픈은 처음·끝·연속 불가)</span>';
    return `<div class="card"><div class="editor">
      <div class="field"><label for="new-name">이름 <span class="meta">${c.name.length}/64</span></label>
        <input class="inp mono wide" id="new-name" data-model="creating.name" value="${esc(c.name)}" placeholder="my-skill" autocomplete="off" spellcheck="false"/>
        <span class="note">${hint}</span></div>
      <div class="field"><label for="new-desc">설명 <span class="meta">${c.description.length}/1024</span></label>
        <textarea class="inp wide" id="new-desc" rows="2" data-model="creating.description" placeholder="무엇을 하고, 언제 쓰는지 적어 주세요. 에이전트가 이 문장을 보고 스킬을 골라요.">${esc(c.description)}</textarea></div>
      <div class="bar"><span class="gap"></span>${btn('createCancel', '취소', { cls: 'ghost' })}${btn('createDo', '만들기', { cls: 'primary', disabled: !valid || !c.description.trim(), id: 'create-do' })}</div>
    </div></div><div class="spacer"></div>`;
  }

  function pageLibrary() {
    const list = filtered();
    const tags = [...new Set(S.snap.skills.flatMap(s => s.tags))].sort();
    const total = S.snap.skills.length;
    const body = !total
      ? `<div class="card"><div class="empty"><b>아직 스킬이 없어요</b>새로 만들거나, 폴더·ZIP을 가져오거나, 설치 위치에 있는 스킬을 가져와 보세요.<br/>스킬 폴더나 ZIP을 이 창에 끌어다 놓아도 돼요.
          <div class="btns">${btn('createOpen', '새 스킬', { cls: 'primary', icon: 'plus' })}${btn('importFolder', '폴더에서', { icon: 'folder' })}${btn('importZip', 'ZIP에서', { icon: 'zip' })}</div></div></div>`
      : list.length ? `<div class="grid-cards">${list.map(skillCard).join('')}</div>`
        : `<div class="card"><div class="empty"><b>찾는 스킬이 없어요</b>검색어나 필터를 바꿔 보세요.</div></div>`;
    return `<div class="page">
      <div class="page-head"><div class="t"><h1>라이브러리</h1><p>스킬 원본을 모아 두는 곳이에요. 여기서 고치고, 필요한 곳에 설치해요.</p></div>
        <div class="actions">${btn('importFolder', '폴더에서', { icon: 'folder' })}${btn('importZip', 'ZIP에서', { icon: 'zip' })}${btn('createOpen', '새 스킬', { cls: 'primary', icon: 'plus' })}</div></div>
      ${S.creating ? createCard() : ''}
      ${total ? `<div class="toolbar">
        <div class="search">${I.search}<input class="inp" id="q" data-model="q" value="${esc(S.q)}" placeholder="이름·설명·태그 검색" autocomplete="off" spellcheck="false"/></div>
        ${seg('filter', [['all', '전체'], ['installed', '설치됨'], ['none', '설치 안 됨'], ['updates', '할 일'], ['issues', '문제']], S.filter)}
        <span class="gap"></span>${seg('sort', [['name', '이름순'], ['updated', '최근 수정']], S.settings.sort)}
      </div>
      ${tags.length ? `<div class="chips toolbar">${tags.map(t => `<button class="chip ${S.tag === t ? 'on' : ''}" data-act="tagFilter" data-arg="${esc(t)}">${esc(t)}</button>`).join('')}</div>` : ''}` : ''}
      ${body}
      ${total ? `<div class="spacer"></div><div class="tip-line">${I.info}<span>스킬 폴더·ZIP·.skill 파일을 창에 끌어다 놓으면 바로 가져와요. 라이브러리 폴더: <span class="mono">${esc(S.snap.libraryDir)}</span></span></div>` : ''}
    </div>`;
  }

  // ───────────────────────── 스킬 상세 ─────────────────────────
  function targetActions(s, t) {
    const st = s.installs[t.id];
    const a = (act, label, cls = 'sm', title = '') => btn(act, label, { cls, arg: s.slug, arg2: t.id, title });
    switch (st) {
      case 'notInstalled': return a('install', '설치', 'sm primary');
      case 'synced': return a('uninstall', '제거', 'sm ghost', '설치 위치에서 휴지통으로 옮겨요');
      case 'libraryNewer': return a('install', '업데이트', 'sm primary') + a('uninstall', '제거', 'sm ghost');
      default: return a('pull', '가져오기', 'sm', '설치본 내용을 라이브러리로 가져와요') + a('overwrite', '덮어쓰기', 'sm', '라이브러리 내용으로 덮어써요 (설치본은 휴지통에 백업)');
    }
  }

  function tabInstall(s) {
    const ts = enabledTargets();
    const appSt = s.installs['claude-app'];
    const issues = s.issues.length
      ? `<div class="issues">${s.issues.map(issueLine).join('')}</div>`
      : `<div class="ok-line">${I.check}<span>Claude · Claude Code · Codex 규칙에 모두 맞아요.</span></div>`;
    return `<div class="card flush"><table class="tbl"><colgroup><col class="c-agent"/><col/><col class="c-status"/><col class="c-act"/></colgroup>
        <thead><tr><th></th><th>설치 위치</th><th>상태</th><th class="r">동작</th></tr></thead><tbody>
        ${ts.map(t => `<tr><td>${agentBadge(t.agent)}</td><td class="nm">${esc(t.name)}<small class="mono">${esc(t.path)}/${esc(s.slug)}</small></td>
          <td>${stTag(s.installs[t.id])}</td><td><div class="acts">${targetActions(s, t)}</div></td></tr>`).join('')}
        <tr><td>${agentBadge('claude-app')}</td><td class="nm">Claude 앱<small>claude.ai · 데스크톱 — 설정 › 기능 › 스킬에서 ZIP을 올려요</small></td>
          <td>${stTag(appSt, true)}</td><td><div class="acts">${btn('exportZip', 'ZIP 내보내기', { cls: `sm ${appSt === 'libraryNewer' ? 'primary' : ''}`, arg: s.slug })}${btn('openClaudeApp', '', { cls: 'sm icon', icon: 'link', title: 'Claude 설정 열기' })}</div></td></tr>
        </tbody></table></div>
      ${ts.length ? '' : `<div class="spacer"></div><div class="note">켜진 설치 위치가 없어요. 설치 위치에서 켜 주세요.</div>`}
      <div class="spacer"></div>
      <div class="grid2 fit-tbl">
        <div class="card"><h2>검사</h2>${issues}</div>
        <div class="card"><div class="rows">
          ${row('즐겨찾기', '', sw(`data-fav="${esc(s.slug)}" aria-label="즐겨찾기"`, s.favorite))}
          <div class="row"><div class="lb">태그<small>${s.tags.length ? '' : '분류할 이름을 붙여요'}</small>
            <div class="chips tag-chips">${s.tags.map(t => `<button class="chip on" data-act="tagRemove" data-arg="${esc(t)}" title="빼기">${esc(t)} ✕</button>`).join('')}</div>
            <input class="inp wide" id="tag-input" data-model="tagInput" value="${esc(S.tagInput)}" placeholder="태그 입력 후 Enter" autocomplete="off"/></div></div>
          ${row('파일', '', `<span class="num">${s.files}개 · ${bytes(s.bytes)}</span>`)}
          ${row('마지막 수정', '', `<span>${ago(s.updated)}</span>`)}
        </div></div>
      </div>`;
  }

  function draftFromDetail(d) {
    return { mode: (S.draft && S.draft.mode) || 'form', name: d.name || d.slug, description: d.description, body: d.body.replace(/^\n/, ''), raw: d.raw, dirty: false };
  }

  function tabEdit() {
    const d = S.detail;
    if (!d) return `<div class="card"><div class="empty">불러오는 중…</div></div>`;
    if (!S.draft) S.draft = draftFromDetail(d);
    const r = S.draft;
    const issues = S.issues || d.issues;
    const descLen = r.description.length;
    const lines = (r.mode === 'form' ? r.body : r.raw).split('\n').length;
    const fields = r.mode === 'form'
      ? `<div class="field"><label for="ed-name">name <span class="meta">${r.name.length}/64</span></label>
          <input class="inp mono wide" id="ed-name" data-draft="name" value="${esc(r.name)}" autocomplete="off" spellcheck="false"/></div>
        <div class="field"><label for="ed-desc">description <span class="meta ${descLen > 1024 ? 'over' : ''}">${descLen}/1024</span></label>
          <textarea class="inp wide" id="ed-desc" rows="3" data-draft="description">${esc(r.description)}</textarea></div>
        <div class="field"><label for="ed-body">본문 (마크다운) <span class="meta ${lines > 500 ? 'over' : ''}">${lines}줄</span></label>
          <textarea class="inp code wide body" id="ed-body" data-draft="body" spellcheck="false">${esc(r.body)}</textarea></div>`
      : `<div class="field"><label for="ed-raw">SKILL.md 원본 <span class="meta">${lines}줄</span></label>
          <textarea class="inp code wide body" id="ed-raw" data-draft="raw" spellcheck="false">${esc(r.raw)}</textarea></div>`;
    return `<div class="split"><div class="card"><div class="editor">
        <div class="bar">${seg('editMode', [['form', '양식'], ['raw', '원본']], r.mode)}<span class="gap"></span>
          ${r.dirty ? '<span class="dirty-dot" title="저장하지 않은 변경"></span>' : ''}
          ${btn('editRevert', '되돌리기', { cls: 'ghost', disabled: !r.dirty })}${btn('editSave', '저장', { cls: 'primary', disabled: !r.dirty })}</div>
        ${fields}
        <div class="note">${r.mode === 'form' ? 'license·allowed-tools 같은 다른 머리말 항목은 그대로 둬요. 전부 고치려면 ‘원본’을 쓰세요.' : '맨 위 --- 사이가 머리말(YAML)이에요.'} <kbd>Ctrl+S</kbd> 저장</div>
      </div></div>
      <div class="sticky"><div class="card"><h2>검사</h2><div class="issues" id="live-issues">${issuesHtml(issues)}</div>
        <div class="spacer"></div><div class="note">${S.settings.autoSync ? '저장하면 따로 고치지 않은 설치본에도 바로 반영돼요.' : '자동 동기화가 꺼져 있어요. 저장한 뒤 설치 탭에서 업데이트하세요.'}</div></div></div>
    </div>`;
  }
  const issuesHtml = list => list.length ? list.map(issueLine).join('') : `<div class="ok-line">${I.check}<span>문제가 없어요</span></div>`;

  function tabFiles() {
    const d = S.detail;
    if (!d) return `<div class="card"><div class="empty">불러오는 중…</div></div>`;
    const f = S.file;
    const editor = f ? `<div class="card"><div class="editor">
        <div class="bar"><span class="mono">${esc(f.path)}</span>${f.content !== f.orig ? '<span class="dirty-dot"></span>' : ''}<span class="gap"></span>
          ${btn('fileClose', '닫기', { cls: 'ghost' })}${btn('fileSave', '저장', { cls: 'primary', disabled: f.content === f.orig && !f.isNew })}</div>
        <textarea class="inp code wide body" id="file-ed" data-model="file.content" spellcheck="false">${esc(f.content)}</textarea>
      </div></div><div class="spacer"></div>` : '';
    return `${editor}<div class="card flush"><table class="tbl"><colgroup><col/><col class="c-size"/><col class="c-act-sm"/></colgroup>
      <thead><tr><th>경로</th><th class="r">크기</th><th class="r">동작</th></tr></thead><tbody>
      ${d.files.map(x => `<tr class="${x.text ? 'click' : ''}" ${x.text ? `data-act="fileOpen" data-arg="${esc(x.path)}"` : ''}>
        <td class="nm"><span class="mono">${esc(x.path)}</span>${x.text ? '' : '<small>텍스트가 아니라 여기서 열 수 없어요</small>'}</td>
        <td class="r num">${bytes(x.bytes)}</td>
        <td><div class="acts">${x.path === 'SKILL.md' ? btn('tab', '편집', { cls: 'sm', arg: 'edit' }) : (x.text ? btn('fileOpen', '열기', { cls: 'sm', arg: x.path }) : '') + btn('fileDelete', '', { cls: 'sm icon ghost', icon: 'trash', arg: x.path, title: '휴지통으로' })}</div></td></tr>`).join('')}
      </tbody></table></div>
      <div class="spacer"></div><div class="row-inline">${btn('fileNew', '새 파일', { icon: 'plus' })}${btn('openSkillDir', '폴더 열기', { icon: 'folder', arg: d.slug })}
      <span class="note">스크립트·참고 문서·템플릿을 함께 넣을 수 있어요. 본문에서 상대 경로로 링크하세요.</span></div>`;
  }

  function pageSkill() {
    const s = skillBy(S.slug);
    if (!s) return `<div class="page"><button class="back" data-act="goto" data-arg="library">${I.back}라이브러리</button><div class="card"><div class="empty"><b>스킬을 찾지 못했어요</b>지워졌거나 이름이 바뀌었어요.</div></div></div>`;
    const tab = S.tab === 'edit' ? tabEdit() : S.tab === 'files' ? tabFiles() : tabInstall(s);
    return `<div class="page">
      <button class="back" data-act="goto" data-arg="library">${I.back}라이브러리</button>
      <div class="page-head"><div class="t"><h1 class="mono">${esc(s.slug)}</h1><p>${esc(s.description || '설명이 없어요')}</p></div>
        <div class="actions">${btn('openSkillDir', '', { cls: 'icon', icon: 'folder', arg: s.slug, title: '폴더 열기' })}${btn('duplicate', '', { cls: 'icon', icon: 'copy', arg: s.slug, title: '복제' })}${btn('deleteSkill', '', { cls: 'icon danger', icon: 'trash', arg: s.slug, title: '라이브러리에서 지우기' })}${btn('exportZip', 'ZIP 내보내기', { icon: 'zip', arg: s.slug })}</div></div>
      <div class="toolbar">${seg('tab', [['install', '설치'], ['edit', '편집' + (S.draft && S.draft.dirty ? ' •' : '')], ['files', `파일 ${s.files}`]], S.tab)}</div>
      ${tab}
    </div>`;
  }

  // ───────────────────────── 설치 위치 ─────────────────────────
  function targetCard(t) {
    const def = !t.builtin ? '' : '';
    const head = `<div class="t-head">${agentBadge(t.agent, 'lg')}<div class="nmw"><div class="nm">${esc(t.name)}</div><div class="path" title="${esc(t.fullPath)}">${esc(t.path)}</div></div>
      ${btn('openPath', '', { cls: 'sm icon ghost', icon: 'folder', arg: t.fullPath, title: '폴더 열기' })}
      ${btn('targetPath', '경로 바꾸기', { cls: 'sm ghost', arg: t.id })}
      ${t.builtin ? btn('targetReset', '기본값', { cls: 'sm ghost', arg: t.id, title: '기본 경로로 되돌려요' }) : btn('targetRemove', '빼기', { cls: 'sm ghost danger', arg: t.id, title: '목록에서만 빼요. 폴더는 그대로예요' })}
      ${sw(`data-target-on="${esc(t.id)}" aria-label="${esc(t.name)} 켜기"`, t.enabled)}${def}</div>`;
    let body = '';
    if (!t.enabled) body = `<div class="note">꺼져 있어요. 켜면 이 폴더의 스킬을 보여주고 설치할 수 있어요.</div>`;
    else if (!t.exists) body = `<div class="note">폴더가 아직 없어요 — 처음 설치할 때 만들어요.</div>`;
    else if (!t.skills.length) body = `<div class="note">스킬이 없어요. 라이브러리에서 설치해 보세요.</div>`;
    else body = `<table class="tbl"><colgroup><col/><col class="c-status"/><col class="c-act"/></colgroup><tbody>
      ${t.skills.map(k => {
        const a = (act, label, cls = 'sm', title = '') => btn(act, label, { cls, arg: k.dir, arg2: t.id, title });
        const acts = !k.inLibrary
          ? btn('importTarget', '라이브러리로 가져오기', { cls: 'sm', arg: t.id, arg2: k.dir })
          : (k.status === 'libraryNewer' ? a('install', '업데이트', 'sm primary')
            : isAttn(k.status) ? a('pull', '가져오기', 'sm', '설치본 내용을 라이브러리로 가져와요') + a('overwrite', '덮어쓰기', 'sm', '라이브러리 내용으로 덮어써요 (설치본은 휴지통에 백업)') : '')
            + btn('open', '열기', { cls: 'sm ghost', arg: k.dir });
        return `<tr><td class="nm"><span class="mono">${esc(k.dir)}</span>${k.symlink ? ' <span class="tag">링크</span>' : ''}<small>${esc(k.description || '')}</small></td>
          <td>${k.inLibrary ? stTag(k.status) : '<span class="tag">라이브러리에 없음</span>'}${k.errors ? ' <span class="tag crit">오류</span>' : ''}</td>
          <td><div class="acts">${acts}</div></td></tr>`;
      }).join('')}</tbody></table>`;
    return `<div class="card">${head}<div class="spacer"></div>${body}</div>`;
  }

  function pageTargets() {
    const ext = S.snap.counts.external;
    return `<div class="page">
      <div class="page-head"><div class="t"><h1>설치 위치</h1><p>에이전트가 스킬을 읽어 가는 폴더예요. 라이브러리의 스킬을 여기에 복사해서 설치해요.</p></div>
        <div class="actions">${ext ? btn('importAll', `밖에 있는 스킬 ${ext}개 가져오기`, { icon: 'download' }) : ''}${btn('addProject', '프로젝트 추가', { cls: 'primary', icon: 'plus' })}</div></div>
      ${S.snap.targets.map(targetCard).join('')}
      <div class="card"><div class="t-head">${agentBadge('claude-app', 'lg')}<div class="nmw"><div class="nm">Claude 앱</div><div class="path">claude.ai · Claude 데스크톱</div></div>
        ${btn('openClaudeApp', 'Claude 설정 열기', { cls: 'sm', icon: 'link' })}</div>
        <div class="spacer"></div>
        <div class="tip-line">${I.info}<span>Claude 앱은 스킬 폴더가 없어서 ZIP으로 올려요. 스킬 화면에서 <b>ZIP 내보내기</b> → Claude <b>설정 › 기능 › 스킬</b>에서 업로드하세요. 내보낸 뒤 라이브러리가 바뀌면 개요에 알려 드려요.</span></div></div>
      <div class="spacer"></div>
      <div class="note">Claude Code는 <span class="mono">~/.claude/skills</span>, Codex는 <span class="mono">~/.agents/skills</span>(예전 <span class="mono">~/.codex/skills</span>), Gemini CLI는 <span class="mono">~/.gemini/skills</span>, Grok Build는 <span class="mono">~/.grok/skills</span>를 읽어요. 프로젝트는 각각 <span class="mono">.claude</span>·<span class="mono">.agents</span>·<span class="mono">.gemini</span>·<span class="mono">.grok</span> 아래 <span class="mono">skills</span> 폴더예요. 경로가 다르면 ‘경로 바꾸기’로 맞춰 주세요.</div>
    </div>`;
  }

  // ───────────────────────── 모양 ─────────────────────────
  function pageLook() {
    const t = S.settings.theme;
    const b = TH.base(t);
    const cTxt = TH.contrast(b.text, b.bg), cAcc = TH.contrast(b.accent, b.bg);
    const warns = [];
    if (cTxt < 4.5) warns.push(`글자와 배경 대비가 ${cTxt.toFixed(1)}:1이에요 — 4.5:1 아래면 읽기 어려워요`);
    if (cAcc < 3) warns.push(`강조색이 배경과 잘 구분되지 않아요 (${cAcc.toFixed(1)}:1)`);
    const tile = (v, l, pv) => `<button class="mode-tile ${t.mode === v ? 'active' : ''}" data-seg="mode" data-v="${v}"><div class="pv ${pv}"><i></i><i></i></div>${l}</button>`;
    const sws = Object.entries(TH.PRESETS).map(([id, p]) => {
      const c = b.dark ? p.dark : p.light;
      return `<button class="swatch ${t.preset === id ? 'active' : ''}" data-seg="preset" data-v="${id}"><span class="chipset"><i style="background:${c.bg}"></i><i style="background:${c.text}"></i><i style="background:${c.accent}"></i></span>${p.name}</button>`;
    }).join('');
    const custom = t.bg || t.text || t.accent;
    const sample = S.snap.skills[0] || { slug: 'example-skill', description: '미리보기용 스킬이에요.', installs: {}, tags: ['예시'], issues: [], updated: Date.now() };
    return `<div class="page">
      <div class="page-head"><div class="t"><h1>모양</h1><p>배경·글자·강조색 세 가지만 고르면 나머지는 자동으로 맞춰요.</p></div></div>
      <div class="preview-wrap"><div class="grid2"><div>${skillCard(sample).replace('data-act="open"', 'data-act=""')}</div>
        <div class="card"><div class="rows">${row('예시 설정', '설명 문장은 이렇게 보여요', sw('aria-label="예시" data-noop', true))}${row('버튼', '', btn('', '보조', { cls: 'sm' }) + btn('', '주 버튼', { cls: 'sm primary' }))}</div></div></div></div>
      <h2>모드</h2>
      <div class="mode-tiles">${tile('system', '시스템 설정 따라가기', 'sys')}${tile('light', '라이트', 'light')}${tile('dark', '다크', 'dark')}</div>
      <h2>테마</h2>
      <div class="swatches">${sws}</div>
      <h2>직접 고르기 <span class="sub">${custom ? '프리셋 위에 덮어쓴 색이 있어요' : '프리셋 색을 쓰는 중'}</span></h2>
      <div class="card"><div class="rows">
        ${row('배경', '', `<span class="colorf"><input type="color" id="c-bg" data-color="bg" value="${b.bg}"/><span class="mono">${b.bg}</span></span>`)}
        ${row('글자', '', `<span class="colorf"><input type="color" id="c-text" data-color="text" value="${b.text}"/><span class="mono">${b.text}</span></span>`)}
        ${row('강조색', '선택된 메뉴, 주 버튼, 업데이트 표시에만 써요', `<span class="colorf"><input type="color" id="c-accent" data-color="accent" value="${b.accent}"/><span class="mono">${b.accent}</span></span>`)}
        ${row('프리셋 색으로 되돌리기', '', btn('colorReset', '되돌리기', { cls: 'sm', disabled: !custom }))}
      </div>${warns.length ? `<div class="spacer"></div><div class="issues">${warns.map(w => `<div class="warn">${I.alert}<span>${w}</span></div>`).join('')}</div>` : ''}</div>
      <h2>글자</h2>
      <div class="card"><div class="rows">
        ${row('글꼴', '', seg('font', Object.entries(TH.FONTS).map(([id, f]) => [id, f.name]), t.font))}
        ${row('화면 배율', '', seg('zoom', TH.ZOOMS.map(z => [z, z + '%']), t.zoom))}
      </div></div>
    </div>`;
  }

  // ───────────────────────── 알림·단축키 ─────────────────────────
  function pageAlerts() {
    const s = S.settings;
    const hk = S.rec
      ? `<span class="tag attn">새 단축키를 누르세요… (Esc 취소)</span>${btn('recCancel', '취소', { cls: 'sm ghost' })}`
      : `<kbd>${esc(accelLabel(s.hotkey, S.platform))}</kbd>${btn('recStart', '바꾸기', { cls: 'sm', disabled: !s.hotkeyEnabled })}`;
    return `<div class="page">
      <div class="page-head"><div class="t"><h1>알림·단축키</h1><p>알림은 드물게, 중요할 때만 보내요.</p></div></div>
      <h2>단축키</h2>
      <div class="card"><div class="rows">
        ${row('어디서든 검색 열기', '다른 앱을 쓰다가도 이 단축키로 라이브러리를 열고 검색칸에 바로 입력해요. 한 번 더 누르면 숨겨요.', sw('data-set="hotkeyEnabled"', s.hotkeyEnabled))}
        ${row('단축키', 'Ctrl·Alt·Shift 중 하나 이상과 글자·숫자·F키를 같이 눌러요.', hk)}
      </div>${S.hotkeyError ? `<div class="spacer"></div><div class="err">${I.alert}<span>${esc(S.hotkeyError)}</span></div>` : ''}</div>
      <h2>창 안 단축키</h2>
      <div class="card"><div class="rows">
        ${row('검색', '', '<kbd>/</kbd> <kbd>Ctrl+F</kbd>')}
        ${row('새 스킬', '', '<kbd>Ctrl+N</kbd>')}
        ${row('저장 (편집 중)', '', '<kbd>Ctrl+S</kbd>')}
        ${row('뒤로 · 닫기', '', '<kbd>Esc</kbd>')}
      </div></div>
      <h2>알림</h2>
      <div class="card"><div class="rows">
        ${row('새 스킬 알림', '설치 위치에 라이브러리에 없는 스킬이 새로 생기면 알려요 (스킬마다 한 번, 앱을 다시 켜도 같은 걸로 또 울리지 않아요)', sw('data-set="notifyExternal"', s.notifyExternal))}
      </div></div>
    </div>`;
  }

  // ───────────────────────── 일반 ─────────────────────────
  function pageGeneral() {
    const s = S.settings;
    const up = !S.release
      ? '<span class="note">개발 빌드라 업데이트를 확인하지 않아요</span>'
      : S.update ? btn('updateInstall', `v${esc(S.update.version)} 설치하고 다시 시작`, { cls: 'primary' }) : btn('updateCheck', '지금 확인', {});
    return `<div class="page">
      <div class="page-head"><div class="t"><h1>일반</h1><p>라이브러리 폴더, 동기화, 시작·업데이트 설정이에요.</p></div></div>
      <h2>동기화</h2>
      <div class="card"><div class="rows">
        ${row('자동 동기화', '라이브러리에서 고치면 설치된 곳에도 바로 반영해요 (설치 위치에서 따로 고친 설치본은 건드리지 않아요)', sw('data-set="autoSync"', s.autoSync))}
      </div></div>
      <h2>라이브러리 폴더</h2>
      <div class="card"><div class="rows">
        ${row('위치', `<span class="mono">${esc(S.snap.libraryDir)}</span>${s.libraryDir ? '' : ' (기본)'}`, btn('openSpecial', '열기', { cls: 'sm', icon: 'folder', arg: 'library' }) + btn('libraryChange', '바꾸기', { cls: 'sm' }) + (s.libraryDir ? btn('libraryReset', '기본값', { cls: 'sm ghost' }) : ''))}
        ${row('휴지통', '지우거나 덮어쓴 스킬은 여기로 옮겨져요. 실수로 지워도 되살릴 수 있어요.', btn('openSpecial', '휴지통 열기', { cls: 'sm', arg: 'trash' }))}
      </div><div class="spacer"></div><div class="note">Git이나 클라우드 드라이브로 동기화되는 폴더를 고르면 여러 컴퓨터에서 같은 라이브러리를 쓸 수 있어요. 빈 폴더를 고르면 지금 스킬을 복사해 가요.</div></div>
      <h2>시작·창</h2>
      <div class="card"><div class="rows">
        ${row('시스템 시작 시 자동 실행', '켜 두면 트레이에서 설치 위치를 지켜보고 새 스킬을 알려요', sw('data-set="autostart"', s.autostart))}
        ${row('창을 닫으면 트레이로', '끄면 창을 닫을 때 앱이 완전히 꺼져요', sw('data-set="closeToTray"', s.closeToTray))}
      </div></div>
      <h2>업데이트</h2>
      <div class="card"><div class="rows">
        ${row(`지금 버전 v${esc(S.version)}`, S.update ? `새 버전 v${esc(S.update.version)}이 있어요` : '3시간마다 확인해요', up)}
        ${row('자동 업데이트', '새 버전을 받으면 바로 설치하고 다시 시작해요', sw('data-set="autoUpdate"', s.autoUpdate))}
      </div></div>
      <h2>데이터</h2>
      <div class="card"><div class="rows">
        ${row('데이터 폴더', `<span class="mono">${esc(S.dataDir)}</span> — 설정(settings.json)과 설치 기록(installs.json)`, btn('openSpecial', '열기', { cls: 'sm', icon: 'folder', arg: 'data' }))}
      </div></div>
    </div>`;
  }

  // ───────────────────────── 대화상자 ─────────────────────────
  function modalHtml() {
    const m = S.modal;
    if (!m) return '';
    return `<div class="modal-back" data-act="modalBack"><div class="modal" role="dialog" aria-modal="true">
      <h3>${esc(m.title)}</h3>${m.msg ? `<p>${m.msg}</p>` : ''}
      ${m.input ? `<input class="inp wide mono" id="modal-input" data-model="modal.input.value" value="${esc(m.input.value)}" placeholder="${esc(m.input.placeholder || '')}" autocomplete="off" spellcheck="false"/>` : ''}
      ${m.seg ? seg('modalSeg', m.seg.options, m.seg.value) : ''}
      ${m.chips ? `<div class="chips">${m.chips.options.map(([v, l]) => `<button class="chip ${m.chips.value.includes(v) ? 'on' : ''}" data-act="modalChip" data-arg="${esc(v)}">${agentBadge(v)}${esc(l)}</button>`).join('')}</div>` : ''}
      ${m.check ? `<label class="chk"><input type="checkbox" id="modal-check" ${m.check.on ? 'checked' : ''}/> ${esc(m.check.label)}</label>` : ''}
      <div class="btns">${btn('modalCancel', m.cancel || '취소', { cls: 'ghost' })}${m.alt ? btn('modalAlt', m.alt, {}) : ''}${btn('modalOk', m.ok || '확인', { cls: m.danger ? 'danger solid' : 'primary', id: 'modal-ok' })}</div>
    </div></div>`;
  }

  function ask(opts) {
    return new Promise(resolve => {
      S.modal = Object.assign({}, opts, { resolve });
      S.lastInput = opts.input ? opts.input.value : '';
      S.lastSeg = opts.seg ? opts.seg.value : '';
      S.lastChips = opts.chips ? opts.chips.value.slice() : [];
      renderModal();
      setTimeout(() => { const el = document.getElementById(opts.input ? 'modal-input' : 'modal-ok'); el && el.focus(); }, 0);
    });
  }
  function closeModal(result) {
    const m = S.modal;
    if (!m) return;
    S.modal = null;
    renderModal();
    m.resolve(result);
  }
  function renderModal() { $modal.innerHTML = modalHtml(); }

  // ───────────────────────── 그리기 ─────────────────────────
  function page() {
    switch (S.page) {
      case 'library': return pageLibrary();
      case 'skill': return pageSkill();
      case 'targets': return pageTargets();
      case 'look': return pageLook();
      case 'alerts': return pageAlerts();
      case 'general': return pageGeneral();
      default: return pageOverview();
    }
  }

  function render() {
    if (!S.settings || !S.snap) return;
    const main = document.getElementById('main');
    const key = S.page + '/' + (S.slug || '') + '/' + S.tab;
    const scroll = main && key === S.lastKey ? main.scrollTop : 0;
    const a = document.activeElement;
    let focus = null;
    if (a && a.id && $app.contains(a)) {
      focus = { id: a.id };
      try { focus.sel = [a.selectionStart, a.selectionEnd]; focus.top = a.scrollTop; } catch (e) { /* 선택 없는 요소 */ }
    }
    $app.innerHTML = titlebar() + `<div class="body">${side()}<main class="main" id="main">${page()}</main></div>`;
    S.lastKey = key;
    const m2 = document.getElementById('main');
    if (m2) m2.scrollTop = scroll;
    if (focus) {
      const el = document.getElementById(focus.id);
      if (el) {
        el.focus({ preventScroll: true });
        try {
          if (focus.sel && focus.sel[0] != null) el.setSelectionRange(focus.sel[0], focus.sel[1]);
          if (focus.top != null) el.scrollTop = focus.top;
        } catch (e) { /* 무시 */ }
      }
    }
  }

  // ───────────────────────── 이동 ─────────────────────────
  async function leaveOk() {
    if (S.rec) recEnd(false);
    const dirtyDraft = S.draft && S.draft.dirty;
    const dirtyFile = S.file && S.file.content !== S.file.orig;
    if (!dirtyDraft && !dirtyFile) return true;
    const r = await ask({ title: '저장하지 않은 변경이 있어요', msg: '저장할까요? 버리면 되돌릴 수 없어요.', ok: '저장', alt: '버리기', cancel: '계속 편집' });
    if (r === 'ok') {
      if (dirtyDraft && !(await saveDraft())) return false;
      if (dirtyFile && !(await saveFile())) return false;
      return true;
    }
    if (r === 'alt') { S.draft = null; S.file = null; S.issues = null; return true; }
    return false;
  }

  async function goto(pg) {
    if (!(await leaveOk())) return;
    S.page = pg;
    if (pg !== 'skill') { S.slug = null; S.detail = null; S.draft = null; S.file = null; S.issues = null; }
    render();
  }

  async function openSkill(slug, tab) {
    if (S.slug !== slug && !(await leaveOk())) return;
    if (S.slug !== slug) { S.detail = null; S.draft = null; S.file = null; S.issues = null; S.tagInput = ''; }
    S.page = 'skill'; S.slug = slug; S.tab = tab || 'install';
    render();
    await loadDetail();
  }

  async function loadDetail() {
    if (!S.slug) return;
    const d = await run(() => call('skill_get', { slug: S.slug }));
    if (d && d.slug === S.slug) {
      S.detail = d;
      if (S.draft && !S.draft.dirty) S.draft = draftFromDetail(d);
      render();
    }
  }
  const reloadDetail = debounce(() => { if (S.page === 'skill' && !(S.draft && S.draft.dirty)) loadDetail(); }, 300);

  // ───────────────────────── 편집 ─────────────────────────
  function draftRaw() {
    const r = S.draft;
    if (r.mode === 'raw') return r.raw;
    return `---\nname: ${JSON.stringify(r.name)}\ndescription: ${JSON.stringify(r.description)}\n---\n${r.body}`;
  }
  const liveCheck = debounce(async () => {
    if (!S.draft || !S.slug) return;
    const issues = await run(() => call('skill_check', { slug: S.slug, raw: draftRaw() }));
    if (!issues) return;
    S.issues = issues;
    const box = document.getElementById('live-issues');
    if (box) box.innerHTML = issuesHtml(issues);
  }, 250);

  async function saveDraft() {
    const r = S.draft;
    if (!r || !r.dirty) return true;
    const res = await run(() => r.mode === 'form'
      ? call('skill_save_fields', { slug: S.slug, name: r.name, description: r.description, body: r.body })
      : call('skill_save', { slug: S.slug, raw: r.raw }));
    if (!res) return false;
    take(res);
    const renamed = res.slug !== S.slug;
    S.slug = res.slug;
    S.draft = null; S.issues = null;
    let msg = renamed ? `저장하고 이름을 ${res.slug}(으)로 바꿨어요` : '저장했어요';
    if (res.synced.length) msg += ` · ${[...new Set(res.synced)].join(', ')}에도 반영했어요`;
    toast(res.errors.length ? res.errors[0] : msg, res.errors.length > 0);
    await loadDetail();
    return true;
  }

  async function saveFile() {
    const f = S.file;
    if (!f) return true;
    const res = await run(() => call('file_write', { slug: S.slug, path: f.path, content: f.content }));
    if (!res) return false;
    take(res);
    S.file = Object.assign({}, f, { orig: f.content, isNew: false });
    toast(res.synced.length ? `저장했어요 · ${[...new Set(res.synced)].join(', ')}에도 반영했어요` : '저장했어요');
    await loadDetail();
    return true;
  }

  // ───────────────────────── 단축키 녹음 (HANDOFF §5.5) ─────────────────────────
  async function recStart() {
    S.rec = true;
    await run(() => call('hotkey_suspend'));
    render();
  }
  async function recEnd(save, accel) {
    if (!S.rec) return;
    S.rec = false;
    if (save && accel) await run(async () => { S.settings = await api.setSettings({ hotkey: accel }); });
    const err = await run(() => call('hotkey_resume'));
    S.hotkeyError = err || null;
    if (save && accel && !err) toast(`단축키를 ${accelLabel(accel, S.platform)}(으)로 바꿨어요`);
    render();
  }

  // ───────────────────────── 동작 ─────────────────────────
  async function setSettings(patch) {
    const s = await run(() => api.setSettings(patch));
    if (s) { S.settings = s; TH.apply(s.theme); render(); }
  }

  const A = {
    goto: el => goto(el.dataset.arg),
    open: el => el.dataset.arg && openSkill(el.dataset.arg),
    openEdit: el => openSkill(el.dataset.arg, 'edit'),
    refresh: async () => { const s = await run(() => call('refresh')); if (s) { S.snap = s; render(); toast('다시 검사했어요'); } },
    winMin: () => run(() => call('win_minimize')),
    winMax: () => run(() => call('win_toggle_maximize')),
    winClose: async () => { if (await leaveOk()) run(() => call('win_close')); },

    install: async el => {
      const r = await run(() => call('install', { slug: el.dataset.arg, target: el.dataset.arg2 }));
      if (r) { S.snap = r; render(); toast('설치했어요'); }
    },
    overwrite: async el => {
      const ok = await ask({ title: '라이브러리 내용으로 덮어쓸까요?', msg: '설치 위치에서 바뀐 내용은 휴지통에 백업해 둬요.', ok: '덮어쓰기', danger: true });
      if (ok === 'ok') A.install(el);
    },
    uninstall: async el => {
      const r = await run(() => call('uninstall', { slug: el.dataset.arg, target: el.dataset.arg2 }));
      if (r) { S.snap = r; render(); toast('제거했어요 — 휴지통으로 옮겼어요'); }
    },
    pull: async el => {
      const ok = await ask({ title: '설치본을 라이브러리로 가져올까요?', msg: '라이브러리의 지금 내용은 휴지통에 백업해 둬요.' + (S.settings.autoSync ? ' 다른 설치 위치에도 퍼져요.' : ''), ok: '가져오기' });
      if (ok !== 'ok') return;
      const r = await run(() => call('pull', { slug: el.dataset.arg, target: el.dataset.arg2 }));
      if (r) { take(r); toast('가져왔어요'); render(); reloadDetail(); }
    },
    importTarget: async el => {
      const r = await run(() => call('import_target', { target: el.dataset.arg, dir: el.dataset.arg2 }));
      if (r) { take(r); render(); toast(`${r.slug}을(를) 라이브러리로 가져왔어요`); }
    },
    importAll: async () => {
      const r = await run(() => call('import_all_external'));
      if (r) { take(r); render(); toast(r.errors.length ? r.errors[0] : `${r.slugs.length}개를 가져왔어요`, r.errors.length > 0); }
    },
    syncAll: async () => {
      const r = await run(() => call('sync_all'));
      if (r) { take(r); render(); toast(r.errors.length ? r.errors[0] : `${r.done}곳을 업데이트했어요`, r.errors.length > 0); }
    },
    importFolder: () => importPick('folder'),
    importZip: () => importPick('zip'),
    exportZip: async el => {
      const p = await run(() => call('export_zip', { slug: el.dataset.arg }));
      if (p) {
        toast('ZIP을 만들었어요 — Claude 설정 › 기능 › 스킬에서 올려 주세요');
        const r = await run(() => call('refresh'));
        if (r) { S.snap = r; render(); }
      }
    },
    openClaudeApp: () => run(() => call('open_url', { url: S.snap.claudeAppUrl })),
    openSkillDir: el => run(() => call('open_skill_dir', { slug: el.dataset.arg })),
    openPath: el => run(() => call('open_path', { path: el.dataset.arg })),
    openSpecial: el => run(() => call('open_special', { which: el.dataset.arg })),

    createOpen: async () => {
      if (S.page !== 'library') await goto('library');
      S.creating = S.creating || { name: '', description: '' };
      render();
      setTimeout(() => { const e = document.getElementById('new-name'); e && e.focus(); }, 0);
    },
    createCancel: () => { S.creating = null; render(); },
    createDo: async () => {
      const c = S.creating;
      const r = await run(() => call('skill_create', { name: c.name.trim(), description: c.description.trim() }));
      if (r) { take(r); S.creating = null; toast('만들었어요 — 본문을 채워 주세요'); openSkill(r.slug, 'edit'); }
    },
    duplicate: async el => {
      const r = await run(() => call('skill_duplicate', { slug: el.dataset.arg }));
      if (r) { take(r); toast(`${r.slug}(으)로 복제했어요`); openSkill(r.slug); }
    },
    deleteSkill: async el => {
      const slug = el.dataset.arg;
      const s = skillBy(slug);
      const inst = s ? enabledTargets().filter(t => s.installs[t.id] && s.installs[t.id] !== 'notInstalled').length : 0;
      const r = await ask({
        title: `${slug}을(를) 지울까요?`, msg: '라이브러리에서 휴지통으로 옮겨요.', ok: '지우기', danger: true,
        check: inst ? { label: `설치된 곳(${inst}곳)에서도 지우기 — 따로 고친 설치본은 남겨요`, on: true } : null,
      });
      if (r !== 'ok') return;
      const uninstall = !!(S.lastCheck);
      const snap = await run(() => call('skill_delete', { slug, uninstall }));
      if (snap) { S.snap = snap; S.draft = null; S.file = null; toast('지웠어요 — 휴지통에서 되살릴 수 있어요'); goto('library'); }
    },
    tab: async el => {
      const v = el.dataset.arg || el.dataset.v;
      S.tab = v;
      render();
      if (!S.detail) loadDetail();
    },
    tagFilter: el => { S.tag = S.tag === el.dataset.arg ? null : el.dataset.arg; render(); },
    tagRemove: async el => {
      const s = skillBy(S.slug);
      const r = await run(() => call('skill_meta', { slug: S.slug, tags: s.tags.filter(t => t !== el.dataset.arg) }));
      if (r) { S.snap = r; render(); }
    },

    editSave: () => saveDraft(),
    editRevert: () => { S.draft = draftFromDetail(S.detail); S.issues = null; render(); },

    fileOpen: async el => {
      const path = el.dataset.arg;
      if (S.file && S.file.content !== S.file.orig && !(await leaveOk())) return;
      const content = await run(() => call('file_read', { slug: S.slug, path }));
      if (content != null) { S.file = { path, content, orig: content }; render(); }
    },
    fileClose: async () => { if (S.file && S.file.content !== S.file.orig && !(await leaveOk())) return; S.file = null; render(); },
    fileSave: () => saveFile(),
    fileNew: async () => {
      const r = await ask({ title: '새 파일', msg: '스킬 폴더 안의 경로를 적어 주세요. 하위 폴더도 돼요.', input: { value: 'reference.md', placeholder: 'scripts/run.py' }, ok: '만들기' });
      if (r !== 'ok') return;
      const path = (S.lastInput || '').trim();
      if (!path) return;
      S.file = { path, content: '', orig: '', isNew: true };
      render();
      setTimeout(() => { const e = document.getElementById('file-ed'); e && e.focus(); }, 0);
    },
    fileDelete: async el => {
      const r = await ask({ title: `${el.dataset.arg}을(를) 지울까요?`, msg: '휴지통으로 옮겨요.', ok: '지우기', danger: true });
      if (r !== 'ok') return;
      const res = await run(() => call('file_delete', { slug: S.slug, path: el.dataset.arg }));
      if (res) { take(res); if (S.file && S.file.path === el.dataset.arg) S.file = null; toast('지웠어요'); loadDetail(); }
    },

    addProject: async () => {
      // 기본 선택: 지금 켜 둔 에이전트들
      const on = [...new Set(enabledTargets().map(t => t.agent).filter(a => a !== 'other'))];
      const value = on.length ? on : ['claude-code', 'codex'];
      const r = await ask({
        title: '프로젝트 추가',
        msg: '어떤 에이전트로 쓸지 고르고 프로젝트 폴더를 고르면, 그 안의 스킬 폴더를 설치 위치로 추가해요.',
        chips: { options: [['claude-code', 'Claude Code'], ['codex', 'Codex'], ['gemini', 'Gemini CLI'], ['grok', 'Grok Build']], value },
        ok: '폴더 고르기',
      });
      if (r !== 'ok' || !S.lastChips.length) return;
      const s = await run(() => call('target_add_project', { agent: S.lastChips.join(',') }));
      if (s) { S.settings = s; toast('추가했어요'); const snap = await run(() => call('refresh')); if (snap) S.snap = snap; render(); }
    },
    modalChip: el => {
      const v = el.dataset.arg;
      const c = S.modal && S.modal.chips;
      if (!c) return;
      c.value = c.value.includes(v) ? c.value.filter(x => x !== v) : [...c.value, v];
      S.lastChips = c.value;
      renderModal();
    },
    targetPath: async el => {
      const s = await run(() => call('target_pick_path', { id: el.dataset.arg }));
      if (s) { S.settings = s; const snap = await run(() => call('refresh')); if (snap) S.snap = snap; render(); }
    },
    targetReset: async el => {
      const s = await run(() => call('target_reset_path', { id: el.dataset.arg }));
      if (s) { S.settings = s; const snap = await run(() => call('refresh')); if (snap) S.snap = snap; render(); }
    },
    targetRemove: async el => {
      const r = await ask({ title: '설치 위치를 뺄까요?', msg: '목록에서만 빼요. 폴더 안의 스킬은 그대로 남아요.', ok: '빼기', danger: true });
      if (r !== 'ok') return;
      const s = await run(() => call('target_remove', { id: el.dataset.arg }));
      if (s) { S.settings = s; const snap = await run(() => call('refresh')); if (snap) S.snap = snap; render(); }
    },
    libraryChange: async () => {
      const s = await run(() => call('library_change_dir'));
      if (s) { S.settings = s; const snap = await run(() => call('refresh')); if (snap) S.snap = snap; render(); toast('라이브러리 폴더를 바꿨어요'); }
    },
    libraryReset: () => setSettings({ libraryDir: '' }),
    colorReset: () => setSettings({ theme: { bg: '', text: '', accent: '' } }),

    recStart: () => recStart(),
    recCancel: () => recEnd(false),
    updateCheck: async () => {
      const u = await run(() => call('update_check'));
      if (u === undefined) return;
      S.update = u;
      toast(u ? `v${u.version}이 있어요` : '최신 버전이에요');
      render();
    },
    updateInstall: () => run(() => call('update_install')),

    modalOk: () => { S.lastCheck = !!(document.getElementById('modal-check') || {}).checked; closeModal('ok'); },
    modalAlt: () => closeModal('alt'),
    modalCancel: () => closeModal('cancel'),
    modalBack: (el, e) => { if (e.target === el) closeModal('cancel'); },
  };

  async function importPick(kind) {
    const r = await run(() => call('import_pick', { kind }));
    if (!r) return;
    take(r);
    afterImport(r.slugs, r.errors);
  }
  function afterImport(slugs, errors) {
    if (errors.length && !slugs.length) { toast(errors[0], true); render(); return; }
    toast(errors.length ? `${slugs.length}개를 가져왔어요 · ${errors[0]}` : `${slugs.length}개를 가져왔어요`, errors.length > 0);
    if (slugs.length === 1) openSkill(slugs[0]); else render();
  }

  const segH = {
    filter: v => { S.filter = v; render(); },
    sort: v => setSettings({ sort: v }),
    tab: v => A.tab({ dataset: { v } }),
    editMode: async v => {
      if (!S.draft || S.draft.mode === v || !S.detail) return;
      if (S.draft.dirty && !(await saveDraft())) return;
      S.draft = Object.assign(draftFromDetail(S.detail), { mode: v });
      S.issues = null;
      render();
    },
    mode: v => setSettings({ theme: { mode: v } }),
    preset: v => setSettings({ theme: { preset: v, bg: '', text: '', accent: '' } }),
    font: v => setSettings({ theme: { font: v } }),
    zoom: v => setSettings({ theme: { zoom: Number(v) } }),
    modalSeg: v => { if (S.modal) { S.modal.seg.value = v; S.lastSeg = v; renderModal(); } },
  };

  // ───────────────────────── 이벤트 연결 ─────────────────────────
  document.addEventListener('click', e => {
    const s = e.target.closest('[data-seg]');
    if (s) { e.preventDefault(); const h = segH[s.dataset.seg]; h && h(s.dataset.v); return; }
    const el = e.target.closest('[data-act]');
    if (!el || el.disabled) return;
    const fn = A[el.dataset.act];
    if (!fn) return;
    // 표 안의 버튼을 누르면 행 클릭은 하지 않아요.
    if (el.tagName !== 'BUTTON' && e.target.closest('button') && e.target.closest('button') !== el) return;
    e.preventDefault();
    fn(el, e);
  });

  const setPath = (obj, path, val) => {
    const ks = path.split('.');
    let o = obj;
    for (let i = 0; i < ks.length - 1; i++) o = o[ks[i]];
    o[ks[ks.length - 1]] = val;
  };

  // 한글 입력(IME) 조합 중에는 다시 그리지 않아요 — 다시 그리면 조합 중인 글자가 깨져요.
  function updateMeta(el) {
    const meta = el.closest('.field') && el.closest('.field').querySelector('.meta');
    if (!meta) return;
    const k = el.dataset.draft || el.dataset.model;
    const n = el.value.length;
    if (k === 'name' || k === 'creating.name') meta.textContent = `${n}/64`;
    else if (k === 'description' || k === 'creating.description') { meta.textContent = `${n}/1024`; meta.classList.toggle('over', n > 1024); }
    else { const l = el.value.split('\n').length; meta.textContent = `${l}줄`; meta.classList.toggle('over', l > 500); }
  }

  function onInput(el, composing) {
    if (el.dataset.model) {
      const m = el.dataset.model;
      const wasDirty = S.file && S.file.content !== S.file.orig;
      setPath(S, m, el.value);
      if (m === 'modal.input.value') S.lastInput = el.value;
      if (composing) return;
      if (m === 'q' || m === 'creating.name') render();
      else if (m === 'creating.description') {
        updateMeta(el);
        const b = document.getElementById('create-do');
        const c = S.creating;
        if (b) b.disabled = !(/^[a-z0-9]+(-[a-z0-9]+)*$/.test(c.name) && c.name.length <= 64 && c.description.trim());
      } else if (m === 'file.content' && wasDirty !== (S.file.content !== S.file.orig)) render();
      return;
    }
    if (el.dataset.draft && S.draft) {
      S.draft[el.dataset.draft] = el.value;
      const wasDirty = S.draft.dirty;
      S.draft.dirty = true;
      liveCheck();
      updateMeta(el);
      if (!wasDirty && !composing) render();
      return;
    }
    if (el.dataset.color) {
      const v = el.value.toUpperCase();
      const patch = { theme: { [el.dataset.color]: v } };
      TH.apply(Object.assign({}, S.settings.theme, patch.theme));
      colorSave(patch);
    }
  }
  document.addEventListener('input', e => onInput(e.target, e.isComposing));
  document.addEventListener('compositionend', e => onInput(e.target, false));
  const colorSave = debounce(patch => setSettings(patch), 200);

  document.addEventListener('change', e => {
    const el = e.target;
    if (el.dataset.set) setSettings({ [el.dataset.set]: el.checked });
    else if (el.dataset.targetOn) {
      const targets = S.settings.targets.map(t => t.id === el.dataset.targetOn ? Object.assign({}, t, { enabled: el.checked }) : t);
      setSettings({ targets });
    } else if (el.dataset.fav) {
      run(async () => { const r = await call('skill_meta', { slug: el.dataset.fav, favorite: el.checked }); S.snap = r; render(); });
    }
  });

  document.addEventListener('keydown', e => {
    if (S.rec) {
      e.preventDefault(); e.stopPropagation();
      if (e.key === 'Escape') { recEnd(false); return; }
      const accel = accelFromEvent(e);
      if (accel) recEnd(true, accel);
      return;
    }
    if (e.isComposing) return;
    const mod = e.ctrlKey || e.metaKey;
    const inField = /^(INPUT|TEXTAREA|SELECT)$/.test((e.target && e.target.tagName) || '');
    if (S.modal) {
      if (e.key === 'Escape') { e.preventDefault(); closeModal('cancel'); }
      if (e.key === 'Enter' && e.target.tagName !== 'TEXTAREA') { e.preventDefault(); A.modalOk(); }
      return;
    }
    if (mod && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (S.file) saveFile(); else if (S.draft && S.draft.dirty) saveDraft();
      return;
    }
    if (mod && e.key.toLowerCase() === 'n') { e.preventDefault(); A.createOpen(); return; }
    if ((mod && e.key.toLowerCase() === 'f') || (!inField && e.key === '/')) { e.preventDefault(); focusSearch(); return; }
    if (e.target.id === 'tag-input' && e.key === 'Enter') {
      e.preventDefault();
      const v = S.tagInput.trim();
      if (!v) return;
      const s = skillBy(S.slug);
      run(async () => { const r = await call('skill_meta', { slug: S.slug, tags: [...s.tags, v] }); S.snap = r; S.tagInput = ''; render(); });
      return;
    }
    if (e.target.id === 'new-name' && e.key === 'Enter') { e.preventDefault(); const d = document.getElementById('new-desc'); d && d.focus(); return; }
    if (e.key === 'Escape') {
      if (inField) { e.target.blur(); return; }
      if (S.creating) { S.creating = null; render(); return; }
      if (S.page === 'skill') goto('library');
    }
  }, true);

  // 녹음은 어떤 이탈 경로에서도 취소해요: 포커스 잃음, 숨김, 페이지 닫힘.
  window.addEventListener('blur', () => recEnd(false));
  document.addEventListener('visibilitychange', () => { if (document.hidden) recEnd(false); });
  window.addEventListener('beforeunload', () => { if (S.rec) call('hotkey_resume'); });

  async function focusSearch() {
    if (S.page !== 'library') await goto('library');
    const q = document.getElementById('q');
    if (q) { q.focus(); q.select(); }
  }

  // ───────────────────────── 시작 ─────────────────────────
  async function start() {
    const d = await api.init();
    Object.assign(S, {
      settings: d.settings, snap: d.snapshot, version: d.version, platform: d.platform, release: d.release,
      update: d.update, dataDir: d.dataDir, defaultLibrary: d.defaultLibrary, hotkeyError: d.hotkeyError,
      firstImported: d.firstImported || 0,
    });
    TH.apply(S.settings.theme);
    TH.watch(() => S.settings.theme);
    render();

    api.on('snapshot', snap => {
      S.snap = snap;
      render();
      if (S.page === 'skill') reloadDetail();
    });
    api.on('settings', s => { S.settings = s; TH.apply(s.theme); render(); });
    api.on('event', ev => {
      if (!ev) return;
      if (ev.type === 'toast') toast(ev.msg);
      else if (ev.type === 'focusSearch') focusSearch();
      else if (ev.type === 'drag') $drop.classList.toggle('on', !!ev.over);
      else if (ev.type === 'imported') afterImport(ev.slugs || [], ev.errors || []);
      else if (ev.type === 'update') { S.update = ev.info || null; render(); }
      else if (ev.type === 'hotkey') { S.hotkeyError = ev.error || null; render(); }
    });
  }

  start().catch(e => {
    $app.innerHTML = `<div class="page"><div class="err">${I.alert}<span>앱을 시작하지 못했어요 — ${esc(e && e.message || e)}</span></div></div>`;
  });

  // 미리보기·테스트용
  window.__SL = { S, render, goto, openSkill };
})();
