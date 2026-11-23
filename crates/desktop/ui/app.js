// The Ilvuno window. It reads the local API (docs/api.md) and renders what it answers. Nothing on
// screen is invented: every run, title, number, agent and sentence comes from a response.
//
// The port is a constant, not a setting, because the Tauri CSP (tauri.conf.json) names the same origin.
// Change both together if the daemon moves. ILVUNO_PORT only changes the daemon side.
(() => {
  'use strict';

  const API = 'http://127.0.0.1:4737';
  const LIST_EVERY_MS = 5000;
  const HEALTH_TIMEOUT_MS = 2000;
  const RELOAD_AFTER_EVENT_MS = 300;

  const $ = (sel, root = document) => root.querySelector(sel);
  const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));
  const esc = (v) => String(v ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const fmt = (n) => new Intl.NumberFormat('en-US').format(Number(n) || 0);
  const compact = (n) => (Number(n) >= 1000 ? Math.floor(Number(n) / 1000) + 'k' : fmt(n));
  const money = (n) => '$' + (Number(n) || 0).toFixed(2);
  const ic = (name, cls = 'ic') => `<svg class="${cls}" width="16" height="16" aria-hidden="true"><use href="#i-${name}"></use></svg>`;
  const chip = (text, cls = '') => `<span class="chip ${cls}">${esc(text)}</span>`;

  class ApiError extends Error {
    constructor(status, message, kind) {
      super(message);
      this.name = 'ApiError';
      this.status = status;
      this.kind = kind;
    }
  }

  // ---- state -----------------------------------------------------------------------------------

  const state = {
    health: null, // null = not asked yet, true = /health answered, false = it did not
    version: null,
    conn: 'connecting', // open | reconnecting | offline | connecting
    runs: [],
    runsError: '',
    currentId: null,
    detail: null,
    board: '',
    boardError: '',
    tab: 'board',
    settings: null,
    settingsError: '',
    agents: null,
    agentsError: '',
    agentsLoading: false,
    settingsTab: 'general',
    settingsRoute: '',
    drawerIndex: -1,
    expanded: new Set(),
    noteDrafts: new Map(),
    opener: null,
    lastDialog: null,
    lastFocusKey: '',
    lastRunId: null,
    toastTimer: 0,
    pollBusy: false,
    gateBusy: false,
    startBusy: false,
    es: null,
    esRunId: null,
    reloadTimer: 0,
    loadSeq: 0,
  };

  // ---- theme (painted from the local cache before the API answers) -----------------------------

  function resolveTheme(pref) {
    if (pref === 'system') return matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
    return pref === 'light' ? 'light' : 'dark';
  }

  function applyTheme() {
    const pref = state.settings?.theme || cachedTheme() || 'dark';
    document.documentElement.dataset.theme = resolveTheme(pref);
    try {
      localStorage.setItem('ilvuno-theme', pref);
    } catch (err) {
      // A window with storage disabled still works; the theme just will not survive a restart.
    }
  }

  function cachedTheme() {
    try {
      return localStorage.getItem('ilvuno-theme');
    } catch (err) {
      return null;
    }
  }

  document.documentElement.dataset.theme = resolveTheme(cachedTheme() || 'dark');
  matchMedia('(prefers-color-scheme: light)').addEventListener('change', () => {
    if (state.settings?.theme === 'system') applyTheme();
  });

  // ---- the API ---------------------------------------------------------------------------------

  async function getJson(path) {
    const res = await fetch(API + path, { headers: { accept: 'application/json' } });
    return readJson(res);
  }

  async function postJson(path, body) {
    const res = await fetch(API + path, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: typeof body === 'string' ? body : JSON.stringify(body),
    });
    return readJson(res);
  }

  async function putJson(path, body) {
    const res = await fetch(API + path, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    });
    return readJson(res);
  }

  async function readJson(res) {
    const text = await res.text();
    let data = null;
    try {
      data = text ? JSON.parse(text) : null;
    } catch (err) {
      data = null;
    }
    if (!res.ok) {
      throw new ApiError(res.status, data?.error || `HTTP ${res.status} ${res.statusText}`, data?.kind);
    }
    return data;
  }

  async function healthCheck() {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), HEALTH_TIMEOUT_MS);
    try {
      const res = await fetch(API + '/health', { signal: controller.signal, headers: { accept: 'application/json' } });
      if (!res.ok) return null;
      return await res.json();
    } catch (err) {
      return null;
    } finally {
      clearTimeout(timer);
    }
  }

  // ---- selection and refresh -------------------------------------------------------------------

  function hasRun(id) {
    return !!id && state.runs.some((r) => r.id === id);
  }

  function pickRun() {
    if (!hasRun(state.currentId)) {
      state.currentId = state.runs[0]?.id || null;
      state.detail = null;
      state.board = '';
      state.drawerIndex = -1;
    }
  }

  async function refreshRuns() {
    if (state.health === false) return;
    try {
      state.runs = await getJson('/runs');
      state.runsError = '';
    } catch (err) {
      state.runsError = err.message;
    }
    pickRun();
    renderAll();
  }

  async function loadRun(id, { refresh = false } = {}) {
    if (!id) return;
    const seq = ++state.loadSeq;
    let detail;
    let board;
    try {
      detail = await getJson(`/runs/${encodeURIComponent(id)}`);
    } catch (err) {
      if (seq !== state.loadSeq) return;
      if (err.status === 404) {
        state.currentId = null;
        state.detail = null;
        state.board = '';
        state.runs = state.runs.filter((r) => r.id !== id);
        pickRun();
        renderAll();
        showToast(`No run has that id: ${err.message}`);
        return;
      }
      if (!refresh) showToast(err.message);
      return;
    }
    try {
      board = await getJson(`/runs/${encodeURIComponent(id)}/board`);
      state.boardError = '';
    } catch (err) {
      board = { markdown: '' };
      state.boardError = err.message;
    }
    if (seq !== state.loadSeq || state.currentId !== id) return;
    state.detail = detail;
    state.board = board?.markdown || '';
    renderAll();
  }

  function scheduleRefresh() {
    clearTimeout(state.reloadTimer);
    state.reloadTimer = setTimeout(() => {
      if (state.currentId && state.health !== false) loadRun(state.currentId, { refresh: true });
    }, RELOAD_AFTER_EVENT_MS);
  }

  // 5 s poll: /health first (that is what turns the window red), then the run list, then a detail
  // fallback while the event stream is not open.
  async function poll({ initial = false } = {}) {
    if (state.pollBusy) return;
    state.pollBusy = true;
    try {
      const health = await healthCheck();
      if (!health) {
        markOffline();
        return;
      }
      state.version = health.version || state.version;
      if (state.health === false) markOnline();
      state.health = true;
      try {
        state.runs = await getJson('/runs');
        state.runsError = '';
      } catch (err) {
        state.runsError = err.message;
      }
      pickRun();
      if (initial && state.currentId) {
        connectEvents(state.currentId);
        await loadRun(state.currentId);
      } else {
        renderAll();
        if (state.currentId && (!state.es || state.es.readyState !== EventSource.OPEN)) {
          loadRun(state.currentId, { refresh: true });
          connectEvents(state.currentId);
        }
      }
    } finally {
      state.pollBusy = false;
    }
  }

  function markOffline() {
    state.health = false;
    state.conn = 'offline';
    closeEvents();
    renderAll();
  }

  function markOnline() {
    state.health = true;
    state.conn = 'reconnecting';
    if (!state.settings || state.settingsError) loadSettings();
    if (state.currentId) {
      connectEvents(state.currentId);
      loadRun(state.currentId, { refresh: true });
    }
    renderAll();
  }

  // ---- event stream ----------------------------------------------------------------------------

  function connectEvents(id) {
    if (!id || state.health === false) return;
    if (state.es && state.esRunId === id && state.es.readyState !== EventSource.CLOSED) return;
    closeEvents();
    const es = new EventSource(`${API}/runs/${encodeURIComponent(id)}/events`);
    state.es = es;
    state.esRunId = id;
    es.onopen = () => {
      state.conn = 'open';
      renderConn();
      scheduleRefresh();
    };
    es.onerror = () => {
      state.conn = state.health === false ? 'offline' : 'reconnecting';
      renderConn();
    };
    es.addEventListener('journal', scheduleRefresh);
  }

  function closeEvents() {
    if (state.es) state.es.close();
    state.es = null;
    state.esRunId = null;
    if (state.health !== false) state.conn = 'reconnecting';
  }

  // ---- view helpers ----------------------------------------------------------------------------

  function hashText() {
    try {
      return decodeURIComponent(location.hash.replace(/^#/, ''));
    } catch (err) {
      return location.hash.replace(/^#/, '');
    }
  }

  function routeName() {
    const h = hashText();
    if (h.startsWith('settings')) return 'settings';
    if (h === 'palette' || h === 'newrun') return h;
    if (h === 'drawer' || /^task-\d+$/.test(h)) return 'drawer';
    if (h === 'tasks' || h === 'log') return h;
    return 'board';
  }

  function settingsTabName() {
    const m = ('#' + hashText()).match(/^#settings-(general|agents|defaults|about)$/);
    return m ? m[1] : 'general';
  }

  function dialogName() {
    const name = routeName();
    return name === 'palette' || name === 'settings' || name === 'newrun' || name === 'drawer' ? name : null;
  }

  function dialogElement() {
    switch (dialogName()) {
      case 'toggle-facts': {
        const app = document.getElementById('app');
        const open = app.dataset.facts !== 'open';
        app.dataset.facts = open ? 'open' : '';
        target.setAttribute('aria-expanded', String(open));
        break;
      }
      case 'palette': return $('#paletteModal');
      case 'settings': return $('#settingsModal');
      case 'newrun': return $('#newrunModal');
      case 'drawer': return $('#drawer');
      default: return null;
    }
  }

  function setRoute(name) {
    const url = name && name !== 'board' ? '#' + name : location.pathname + location.search;
    try {
      history.replaceState(null, '', url);
    } catch (err) {
      location.hash = name && name !== 'board' ? name : '';
    }
    applyRoute();
  }

  function applyRoute() {
    const name = routeName();
    if (name === 'tasks' || name === 'log') state.tab = name;
    else if (name === 'board' || name === 'drawer') state.tab = state.tab === 'tasks' || state.tab === 'log' ? state.tab : 'board';
    if (name === 'drawer') {
      const m = hashText().match(/^task-(\d+)$/);
      const tasks = state.detail?.tasks || [];
      state.drawerIndex = m ? Math.min(tasks.length, Math.max(1, Number(m[1]))) - 1 : (tasks.length ? tasks.length - 1 : -1);
    }
    if (name === 'settings') state.settingsTab = settingsTabName();
    if (name === 'palette') openPalette();
    renderAll();
  }

  function openDialog(name) {
    state.opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setRoute(name);
  }

  function closeDialog() {
    setRoute(state.tab === 'tasks' || state.tab === 'log' ? state.tab : 'board');
  }

  function projectPath() {
    return state.detail?.run?.project || state.runs.find((r) => r.id === state.currentId)?.project || '';
  }

  function projectName(path = projectPath()) {
    const name = String(path || '').replace(/\/+$/, '').split('/').filter(Boolean).pop();
    return name || 'project';
  }

  function shortProject(path = projectPath()) {
    return String(path || '').replace(/^\/home\/[^/]+/, '~');
  }

  // Paths are shown relative to the repo root, the same way the export and the v3 mock do it.
  function rootPath() {
    const project = projectPath();
    return project ? project.replace(/\/[^/]+$/, '') : '';
  }

  function relPath(p) {
    let s = String(p || '');
    const root = rootPath();
    if (root && s.startsWith(root + '/')) s = s.slice(root.length + 1);
    s = s.replace(/^.*\/runs\/[0-9a-f-]{36}\//, 'run/');
    return s;
  }

  function shrinkText(text) {
    let s = String(text ?? '');
    const root = rootPath();
    if (root) s = s.split(root + '/').join('');
    s = s.replace(/\/(?:[\w.\-]+\/)*runs\/([0-9a-f\-]{36})\//g, 'run/');
    return s;
  }

  function firstSentence(text) {
    const s = String(text ?? '').trim();
    const m = s.match(/^(.+?[.!?])(\s|$)/);
    return m ? m[1] : s;
  }

  function runTitle() {
    const raw = String(state.detail?.goal?.text || state.runs.find((r) => r.id === state.currentId)?.title || 'Untitled run');
    const cut = raw.indexOf('. ');
    return (cut > 0 ? raw.slice(0, cut) : raw).replace(/\.$/, '');
  }

  function roundsUsed() {
    const d = state.detail;
    return d?.run?.rounds_used ?? d?.rounds?.length ?? 0;
  }

  function statusWord(status) {
    return String(status || 'unknown').replace(/_/g, ' ');
  }

  function statusClass(status) {
    if (['accepted', 'merged', 'pass', 'pass_with_notes'].includes(status)) return 'ok';
    if (['blocked', 'budget_exhausted', 'no_progress', 'failed', 'block'].includes(status)) return 'bad';
    if (status === 'reviewed' || status === 'human_paused') return 'warn';
    return '';
  }

  function statusChip(status) {
    return chip(statusWord(status), statusClass(status));
  }

  function safetyForAgent(agent) {
    const name = String(agent || '');
    if (/claude/i.test(name)) return { label: 'asks first', cls: 'safe', note: 'Asks before every tool.' };
    if (/codex|pi|opencode|agy|antigravity|cursor/i.test(name)) {
      return { label: 'folder only', cls: 'safe', note: 'Writes inside its folder, asks before anything outside.' };
    }
    return { label: 'unknown', cls: '', note: 'This agent has not declared how it asks for permission.' };
  }

  function safetyLine(safety) {
    if (safety === 'asks_before_each_tool') return 'Asks before every tool';
    if (safety === 'works_in_folder_asks_outside') return 'Writes inside its folder, asks before anything outside';
    return 'Permissions unknown';
  }

  // ---- toast -----------------------------------------------------------------------------------

  function showToast(message) {
    const el = $('#toast');
    if (!el) return;
    el.textContent = message;
    el.classList.add('on');
    clearTimeout(state.toastTimer);
    state.toastTimer = setTimeout(() => el.classList.remove('on'), 3200);
  }

  // ---- inert and focus -------------------------------------------------------------------------

  function setInert(el, on) {
    if (!el) return;
    if ('inert' in HTMLElement.prototype) {
      el.inert = on;
      return;
    }
    // Older WebKit: keep it out of the tab order and the accessibility tree by hand.
    if (on) {
      el.setAttribute('aria-hidden', 'true');
      $$('a[href],button,input,textarea,select,[tabindex]', el).forEach((node) => {
        if (node.hasAttribute('data-prev-tab')) return;
        node.dataset.prevTab = node.getAttribute('tabindex') ?? '';
        node.setAttribute('tabindex', '-1');
      });
    } else {
      el.removeAttribute('aria-hidden');
      $$('[data-prev-tab]', el).forEach((node) => {
        const prev = node.dataset.prevTab;
        if (prev === '') node.removeAttribute('tabindex');
        else node.setAttribute('tabindex', prev);
        delete node.dataset.prevTab;
      });
    }
  }

  function applyInert() {
    const app = $('#app');
    if (!app) return;
    if (dialogElement()) {
      setInert(app, true);
      return;
    }
    setInert(app, false);
    const offline = state.health === false;
    setInert($('.rail'), offline);
    setInert($('.facts'), offline);
    setInert($('#scroll'), offline);
  }

  function focusables(root) {
    return $$('a[href],button:not([disabled]),input:not([disabled]),textarea:not([disabled]),select:not([disabled]),[tabindex]:not([tabindex="-1"])', root)
      .filter((el) => el.getClientRects().length > 0 && !el.closest('[inert]'));
  }

  function trapTab(e, root) {
    const f = focusables(root);
    if (!f.length) {
      e.preventDefault();
      root.focus();
      return;
    }
    const first = f[0];
    const last = f[f.length - 1];
    const active = document.activeElement;
    if (e.shiftKey) {
      if (active === first || !root.contains(active)) {
        e.preventDefault();
        last.focus();
      }
    } else if (active === last || !root.contains(active)) {
      e.preventDefault();
      first.focus();
    }
  }

  function focusDialogOnce(name) {
    const key = `${name}|${name === 'settings' ? settingsTabName() : ''}|${name === 'drawer' ? state.drawerIndex : ''}`;
    if (state.lastFocusKey === key) return;
    state.lastFocusKey = key;
    const modal = dialogElement();
    if (!modal) return;
    const go = () => {
      if (!document.contains(modal) || !modal.classList.contains('open')) return;
      if (name === 'palette') {
        $('#palInput')?.focus({ preventScroll: true });
      } else if (name === 'drawer') {
        modal.focus({ preventScroll: true });
      } else if (name === 'newrun') {
        $('#nr-goal')?.focus({ preventScroll: true });
      } else {
        ($('.set-tab.on') || focusables(modal)[0] || modal).focus({ preventScroll: true });
      }
    };
    go();
    setTimeout(go, 40);
  }

  function restoreOpener(closedName) {
    const opener = state.opener;
    state.opener = null;
    const usable = opener && document.contains(opener) && opener.getClientRects().length && !opener.closest('[inert]');
    let target = usable ? opener : null;
    if (!target) {
      target = closedName === 'drawer'
        ? $(`.task[data-i="${state.drawerIndex}"]`)
        : closedName === 'palette'
          ? $('[data-act="palette"]')
          : closedName === 'settings'
            ? $('[data-act="settings"]')
            : $('.newrun');
    }
    if (target) setTimeout(() => target.focus({ preventScroll: true }), 0);
  }

  // ---- rail ------------------------------------------------------------------------------------

  function runDotClass(status) {
    const cls = statusClass(status);
    return cls === 'ok' ? 'ok' : cls === 'bad' ? 'bad' : cls === 'warn' ? 'warn' : '';
  }

  function railRunRow(run) {
    const on = run.id === state.currentId;
    return `
      <button class="run-row${on ? ' on' : ''}" data-act="select-run" data-id="${esc(run.id)}"${on ? ' aria-current="true"' : ''}>
        <div class="run-title">
          <span class="dot ${runDotClass(run.status)}"></span>
          <span class="t">${esc(run.title || 'Untitled run')}</span>
        </div>
        <div class="run-meta">${esc(statusWord(run.status))}<span class="sep"> · </span>${fmt(run.rounds_used)} rounds<span class="sep"> · </span>${compact(run.tokens_used)}</div>
      </button>`;
  }

  function renderRail() {
    const body = $('#railBody');
    const foot = $('#railFoot');
    if (!body || !foot) return;

    const today = new Date();
    const isToday = (ms) => {
      const d = new Date(ms);
      return d.getFullYear() === today.getFullYear() && d.getMonth() === today.getMonth() && d.getDate() === today.getDate();
    };
    const todays = state.runs.filter((r) => isToday(r.started_at));
    const earlier = state.runs.filter((r) => !isToday(r.started_at));

    const projectMap = new Map();
    for (const run of state.runs) {
      const key = run.project || '';
      const entry = projectMap.get(key) || { path: key, count: 0, runId: run.id };
      entry.count += 1;
      projectMap.set(key, entry);
    }

    const sections = [];
    if (todays.length) sections.push(`<div class="section"><span class="lbl">Today</span><span class="n">${todays.length}</span></div><div class="runlist">${todays.map(railRunRow).join('')}</div>`);
    if (earlier.length) sections.push(`<div class="section"><span class="lbl">Earlier</span><span class="n">${earlier.length}</span></div><div class="runlist">${earlier.map(railRunRow).join('')}</div>`);

    body.innerHTML = `
      <button class="cmd" data-act="palette" aria-label="Search runs and actions">
        ${ic('search')}
        <span class="lbl">Search runs</span>
        <span class="kbd">⌘K</span>
      </button>
      <button class="newrun" data-act="new-run">${ic('plus')}New run</button>
      ${state.runsError ? `<p class="rail-error" role="alert">${esc(state.runsError)}</p>` : ''}
      ${state.runs.length ? sections.join('') : '<div class="section"><span class="lbl">Runs</span><span class="n">0</span></div><div class="rail-empty">No runs yet.</div>'}
      ${projectMap.size ? `<div class="section"><span class="lbl">Projects</span><span class="n">${projectMap.size}</span></div>${Array.from(projectMap.values()).map((p) => `
        <button class="proj" data-act="select-run" data-id="${esc(p.runId)}" title="${esc(p.path)}">
          ${ic('folder')}
          <span class="name">${esc(projectName(p.path))}</span>
          <span class="cnt">${p.count} run${p.count === 1 ? '' : 's'}</span>
        </button>`).join('')}` : ''}`;

    const effective = resolveTheme(state.settings?.theme || cachedTheme() || 'dark');
    foot.innerHTML = `
      <button class="iconbtn" data-act="settings" aria-label="Settings" title="Settings">${ic('settings')}</button>
      <button class="theme-btn" data-act="toggle-theme" aria-label="Switch to ${effective === 'dark' ? 'light' : 'dark'} theme" title="Toggle theme">
        ${ic('moon')}<span class="theme-label">${effective === 'dark' ? 'Dark' : 'Light'}</span>
      </button>`;
  }

  // ---- header ----------------------------------------------------------------------------------

  function gateReceipt() {
    const { run, rounds = [] } = state.detail;
    const passedRound = rounds.find((r) => r.verdict === 'pass');
    const checks = passedRound?.readiness?.checks || [];
    const n = checks.filter((c) => c.passed).length;
    const at = passedRound ? passedRound.index : Math.max(0, roundsUsed() - 1);
    const verb = run.status === 'merged' ? 'Merged' : 'Accepted';
    return `
      <div class="gate-bar" aria-label="Gate ${verb.toLowerCase()}">
        <span class="gate-ic">${ic('check')}</span>
        <div class="gate-main">
          <span class="gate-k">Gate · closed</span>
          <span class="gate-t">${verb} at round ${at} · ${n} of ${checks.length} checks passed</span>
        </div>
        <span class="gate-tags">${chip(verb.toLowerCase(), 'ok')}</span>
      </div>`;
  }

  function gatePanel() {
    const { rounds = [] } = state.detail;
    const round = rounds[rounds.length - 1] || { index: 0, readiness: { checks: [] } };
    const checks = round.readiness?.checks || [];
    const draft = state.noteDrafts.get(state.currentId) || '';
    return `
      <section class="gate-panel" aria-label="Gate">
        <div class="gate-phead"><span class="gate-k">Gate · round ${esc(round.index)}</span>${chip('open', 'warn')}</div>
        <p class="gate-copy">The checks passed. Nothing has been merged. Accept to record the decision, or send it back with a note the team reads.</p>
        <div class="gate-checks">${checks.length ? checks.map((c) => `
          <div class="gate-check">
            <svg class="${c.passed ? 'ic' : 'ic bad'}" aria-hidden="true"><use href="#i-${c.passed ? 'check' : 'x'}"></use></svg>
            <span>${esc(c.name)}</span>
            ${chip(c.passed ? 'pass' : 'fail', c.passed ? 'ok' : 'bad')}
          </div>`).join('') : '<div class="gate-check">No checks were recorded for this round.</div>'}
        </div>
        <label class="field-label" for="gate-note">Note to the team <span>needed only to send back</span></label>
        <textarea class="gate-note" id="gate-note" rows="2" placeholder="What should change before this is accepted?">${esc(draft)}</textarea>
        <div class="gate-actions">
          <button class="btn primary" data-act="accept-gate"${state.gateBusy ? ' disabled' : ''}>Accept</button>
          <button class="btn" data-act="send-back"${state.gateBusy ? ' disabled' : ''}>Send back with note</button>
        </div>
        <p class="gate-inline" id="gateInline" role="status" hidden></p>
      </section>`;
  }

  function stopPanel() {
    const { run } = state.detail;
    const reason = run.blocked_reason || 'The run stopped and nothing was merged.';
    return `
      <section class="stop" aria-label="Run stopped">
        <div class="stop-head">${ic('alert-triangle')}<span>Stopped: ${esc(statusWord(run.status))}</span></div>
        <p>${esc(reason)}</p>
        <div class="stop-actions">
          <button class="btn" data-act="checks">See what failed ${ic('chevron-right', 'ic sm')}</button>
          <span class="stop-meta">${fmt(run.tokens_used)} tokens · ${roundsUsed()} rounds</span>
        </div>
      </section>`;
  }

  function gateArea() {
    const run = state.detail?.run;
    if (!run) return '';
    if (run.status === 'reviewed') return gatePanel();
    if (['blocked', 'budget_exhausted', 'no_progress', 'human_paused'].includes(run.status)) return stopPanel();
    if (run.status === 'accepted' || run.status === 'merged') return gateReceipt();
    return '';
  }

  function tabsHtml() {
    const ledgerCount = state.detail?.ledger?.length || 0;
    const tasksCount = state.detail?.tasks?.length || 0;
    const lines = state.board ? state.board.split('\n').length : 0;
    const tab = (id, label, n, sr) => `
      <button class="tab" role="tab" id="tab-${id}" data-act="tab" data-tab="${id}" title="${sr}"
        aria-controls="pane-${id}" aria-selected="${state.tab === id}" aria-label="${label}, ${sr}">
        ${label}<span class="cnt" aria-hidden="true">${n}</span>
      </button>`;
    return `
      <div class="tabs" role="tablist" aria-label="Run record">
        ${tab('board', 'Board', ledgerCount, `${ledgerCount} entries`)}
        ${tab('tasks', 'Tasks', tasksCount, `${tasksCount} tasks`)}
        ${tab('log', 'Log', lines, `${lines} lines of board.md`)}
      </div>`;
  }

  // ---- board -----------------------------------------------------------------------------------

  function scopeChips(scope) {
    if (!scope || !scope.length) return '';
    const items = scope.map((p) => p === 'run'
      ? `<span class="fchip run" title="the whole run">whole run</span>`
      : `<span class="fchip" title="${esc(relPath(p))}">${esc(relPath(p))}</span>`).join('');
    return `<div class="led-scope">${items}</div>`;
  }

  function ledgerHtml(e) {
    const kind = ['intent', 'finding', 'question', 'decision'].includes(e.kind) ? e.kind : '';
    const id = 'led-' + e.id;
    const expanded = state.expanded.has(String(id));
    return `
      <article class="led">
        <div><span class="kind ${kind}">${esc(e.kind || 'entry')}</span></div>
        <div>
          <div class="led-meta">
            <span class="who">by ${esc(e.author || e.thread || 'runner')}</span>
            <span class="sep"> · </span><span>round ${esc(e.round ?? '—')}</span>
          </div>
          <p class="led-body${expanded ? '' : ' clamp'}" id="${id}">${esc(shrinkText(e.body))}</p>
          <button class="led-more" data-act="more" data-target="${id}" aria-expanded="${expanded}" aria-controls="${id}"${expanded ? '' : ''}>${expanded ? 'less' : 'more'}</button>
          ${scopeChips(e.scope)}
        </div>
      </article>`;
  }

  function boardGroups() {
    const { rounds = [], ledger = [] } = state.detail;
    if (!ledger.length) return '<div class="facts-empty">No board entries recorded.</div>';
    const byRound = new Map();
    let loose = [];
    for (const entry of ledger) {
      if (entry.round === null || entry.round === undefined) {
        loose.push(entry);
        continue;
      }
      const key = Number(entry.round);
      byRound.set(key, [...(byRound.get(key) || []), entry]);
    }
    loose = loose.sort((a, b) => Number(a.id) - Number(b.id));
    const indexes = Array.from(new Set([...rounds.map((r) => Number(r.index)), ...byRound.keys()])).sort((a, b) => a - b);
    const groups = indexes
      .filter((i) => (byRound.get(i) || []).length)
      .map((i) => ({
        label: `Round ${i}`,
        verdict: rounds.find((r) => Number(r.index) === i)?.verdict,
        entries: byRound.get(i).sort((a, b) => Number(a.id) - Number(b.id)),
      }));
    if (loose.length) groups.push({ label: 'Run', verdict: null, entries: loose });
    if (!groups.length) return '<div class="facts-empty">No board entries recorded.</div>';
    return groups.map((g) => `
      <section class="rgroup" aria-label="${esc(g.label)}">
        <div class="rgroup-head">
          <span class="rgroup-n">${esc(g.label)}</span>
          <span class="rgroup-c">${g.entries.length} ${g.entries.length === 1 ? 'entry' : 'entries'}</span>
          ${g.verdict ? chip(String(g.verdict).replace(/_/g, ' '), statusClass(g.verdict)) : ''}
        </div>
        ${g.entries.map(ledgerHtml).join('')}
      </section>`).join('');
  }

  function scopeLabel(t) {
    const scope = t.scope || [];
    if (!scope.length) return 'no scope';
    if (scope.length === 1 && scope[0] === 'run') return 'whole run';
    return `${scope.length} files`;
  }

  function tasksHtml() {
    const tasks = state.detail?.tasks || [];
    if (!tasks.length) return '<div class="facts-empty">No tasks recorded.</div>';
    return `<div class="tasks">${tasks.map((t, i) => {
      const report = shrinkText(t.result || '');
      const first = firstSentence(report);
      return `
        <button class="task" data-act="task" data-i="${i}" aria-label="Open task ${i + 1}, ${esc(t.thread)}">
          <div class="task-head">
            <span class="task-num">#${i + 1}</span>
            <span class="task-thread">${esc(t.thread)}</span>
            ${chip(statusWord(t.status || 'reported'), statusClass(t.status))}
            <span class="chev">${ic('chevron-right', 'ic sm')}</span>
          </div>
          <p class="task-prompt">${esc(shrinkText(t.prompt))}</p>
          ${first ? `<p class="task-result" title="${esc(report)}">${esc(first)}</p>` : ''}
          <div class="task-meta"><span>round ${esc(t.round)} · by ${esc(t.created_by || 'runner')}</span><span class="r">${scopeLabel(t)}</span></div>
        </button>`;
    }).join('')}</div>`;
  }

  function emptyState() {
    if (state.health === false) {
      return `
        <div class="empty-state">
          <span class="empty-gem" aria-hidden="true">◆</span>
          <p>Start the runner to see your runs. The window reads it on 127.0.0.1:4737.</p>
        </div>`;
    }
    if (!state.runs.length) {
      return `
        <div class="empty-state">
          <span class="empty-gem" aria-hidden="true">◆</span>
          <p>No runs yet — start one when you have a goal for your agents.</p>
          <div class="empty-actions"><button class="btn primary" data-act="new-run">${ic('plus')}New run</button></div>
        </div>`;
    }
    if (state.currentId && !state.detail) return '<div class="facts-empty">Reading the run…</div>';
    return '<div class="facts-empty">Nothing is selected. Pick a run in the list.</div>';
  }

  function runHeader() {
    const { goal, threads = [], tasks = [] } = state.detail;
    const chips = [
      statusChip(state.detail.run?.status),
      chip(`${roundsUsed()} rounds`, 'num'),
      chip(`${fmt(state.detail.run?.tokens_used)} tokens`, 'num'),
      chip(`${threads.length} threads`, 'num'),
      chip(`${tasks.length} tasks`, 'num'),
    ].join('');
    return `
      <h1>${esc(runTitle())}</h1>
      <div class="head-sub">
        <p class="done" title="${esc(goal?.done || '')}">${esc(goal?.done || 'No done condition recorded.')}</p>
        <div class="chips">${chips}</div>
      </div>`;
  }

  function panesHtml() {
    const hidden = (id) => (state.tab === id ? '' : ' hidden');
    return `
      <section class="pane" role="tabpanel" id="pane-board" aria-labelledby="tab-board" tabindex="0"${hidden('board')}>
        ${boardGroups()}
      </section>
      <section class="pane" role="tabpanel" id="pane-tasks" aria-labelledby="tab-tasks" tabindex="0"${hidden('tasks')}>
        ${tasksHtml()}
      </section>
      <section class="pane" role="tabpanel" id="pane-log" aria-labelledby="tab-log" tabindex="0"${hidden('log')}>
        <div class="logbar"><span>board.md</span>${chip('read only')}<span class="sp">${state.board ? state.board.split('\n').length : 0} lines</span></div>
        ${state.boardError ? `<p class="nr-error" role="alert">${esc(state.boardError)}</p>` : ''}
        <pre class="log-pre" tabindex="0" aria-label="board.md, read only">${esc(state.board || (state.boardError ? '' : 'The board is empty.'))}</pre>
      </section>`;
  }

  function renderBoard() {
    const board = $('#board');
    const scroll = $('#scroll');
    if (!board || !scroll) return;
    const top = scroll.scrollTop;

    if (!state.detail) {
      board.innerHTML = emptyState();
    } else {
      board.innerHTML = `${runHeader()}${gateArea()}${tabsHtml()}${panesHtml()}`;
    }
    if (state.lastRunId === state.currentId) scroll.scrollTop = top;
    else scroll.scrollTop = 0;

    requestAnimationFrame(initExpanders);
  }

  function initExpanders() {
    $$('.led-body.clamp').forEach((p) => {
      const btn = document.querySelector(`[data-act="more"][data-target="${p.id}"]`);
      if (!btn) return;
      const over = p.scrollHeight > p.clientHeight + 1;
      btn.hidden = !over;
      if (!over) p.classList.remove('clamp');
    });
  }

  // ---- facts -----------------------------------------------------------------------------------

  function factsHtml() {
    if (!state.detail) {
      return `
        <div class="fsec">
          <div class="fsec-head"><h2>Project</h2></div>
          <div class="path">${esc(shortProject() || '—')}</div>
          <div class="frow"><span>Runs</span><span class="v">${state.runs.length}</span></div>
        </div>
        <div class="facts-empty">${state.health === false ? 'The runner is not answering, so there is nothing to check.' : 'The board fills in as threads report. Nothing to check yet.'}</div>`;
    }

    const { run, goal, rounds = [], threads = [] } = state.detail;
    const budget = goal?.budget || { rounds: null, tokens: null };
    const checks = rounds.flatMap((r) => (r.readiness?.checks || []).map((c) => ({ ...c, round: r.index })));
    const passed = checks.filter((c) => c.passed).length;
    const pct = budget.tokens ? Math.min(100, Math.round(((Number(run.tokens_used) || 0) / budget.tokens) * 100)) : 0;
    const spent = rounds.reduce((sum, r) => sum + (Number(r.cost_usd) || 0), 0);
    const duration = run.ended_at && run.started_at
      ? Math.floor((run.ended_at - run.started_at) / 60000) + 'm ' + Math.round(((run.ended_at - run.started_at) % 60000) / 1000) + 's'
      : run.started_at ? 'in progress' : '—';

    const checkGroups = rounds.map((r) => `
      <div class="chk-round">Round ${esc(r.index)}</div>
      ${(r.readiness?.checks || []).map((c) => `
        <div class="chk">
          <span class="name" title="${esc(c.detail || c.name)}">${esc(c.name)}</span>
          ${chip(c.passed ? 'pass' : 'fail', c.passed ? 'ok' : 'bad')}
        </div>`).join('') || '<div class="facts-empty">No checks.</div>'}`).join('');

    const roundRows = rounds.map((r) => `
      <div class="frow">
        <span>R${esc(r.index)} · ${esc(statusWord(r.verdict || r.status))}</span>
        <span class="v">${r.tokens != null ? fmt(r.tokens) + ' tok' : '—'}${r.cost_usd != null ? ' · ' + money(r.cost_usd) : ''}</span>
      </div>`).join('');

    const threadRows = threads.map((t) => {
      const s = safetyForAgent(t.agent);
      return `
        <div class="thread">
          <span class="dot ${t.status === 'idle' ? '' : 'ok'}"></span>
          <span class="th-name">${esc(t.name)}</span>
          <span class="chip ${s.cls}" title="${esc(s.note)}">${esc(s.label)}</span>
          <span class="th-agent">${esc(t.agent || 'unknown')}</span>
        </div>`;
    }).join('');

    const artifactList = (state.detail.artifacts || []).slice().sort((a, b) => roundOfArtifact(b) - roundOfArtifact(a));
    const artifactRows = artifactList.map((a) => {
      const parts = String(a.path || '').split('/');
      const fn = parts[parts.length - 1] || a.path;
      const m = String(a.path || '').match(/\/rounds\/r(\d+)\/([^/]+)\//);
      const loc = m ? `r${m[1]} · ${m[2]}` : (a.thread || 'artifact');
      return `
        <div class="art" title="${esc(a.path)}">
          <span class="fn">${esc(fn)}</span>
          <span class="sz">${a.bytes != null ? fmt(a.bytes) + ' b' : '—'}</span>
          <span class="loc">${esc(loc)}</span>
        </div>`;
    }).join('');

    return `
      <div class="fsec">
        <div class="fsec-head"><h2>Project</h2></div>
        <div class="path" title="${esc(run.project || '')}">${esc(shortProject(run.project))}</div>
        <div class="runid">
          <span class="k">Run id</span>
          <code title="${esc(run.id)}">${esc(run.id)}</code>
          <button class="copybtn" data-act="copy-id" aria-label="Copy run id" title="Copy run id">${ic('copy')}</button>
        </div>
        <div class="frow"><span>Duration</span><span class="v">${duration}</span></div>
      </div>
      <div class="fsec">
        <div class="fsec-head"><h2>Rounds</h2><span class="n">${rounds.length}</span></div>
        ${roundRows || '<div class="facts-empty">No rounds recorded.</div>'}
      </div>
      <div class="fsec" id="checks">
        <div class="fsec-head"><h2>Checks · all rounds</h2><span class="n">${passed} / ${checks.length}</span></div>
        ${checkGroups || '<div class="facts-empty">No checks recorded.</div>'}
      </div>
      <div class="fsec">
        <div class="fsec-head"><h2>Budget</h2><span class="n">${pct}%</span></div>
        <div class="bar" role="img" aria-label="${pct}% of the token budget used"><i data-pct="${pct}"></i></div>
        <div class="brow"><span>Tokens</span><span class="v"><b>${fmt(run.tokens_used)}</b> / ${budget.tokens ? fmt(budget.tokens) : '—'}</span></div>
        <div class="brow"><span>Rounds</span><span class="v"><b>${roundsUsed()}</b> / ${budget.rounds ?? '—'}</span></div>
        <div class="brow"><span>Spend</span><span class="v">${money(spent)}</span></div>
      </div>
      <div class="fsec">
        <div class="fsec-head"><h2>Threads</h2><span class="n">${threads.length}</span></div>
        ${threadRows || '<div class="facts-empty">No threads yet.</div>'}
        <div class="legend">Ilvuno does not sandbox agents; each CLI is bounded by its own permissions.</div>
      </div>
      <div class="fsec">
        <div class="fsec-head"><h2>Artifacts</h2><span class="n">${artifactList.length}</span></div>
        ${artifactRows || '<div class="facts-empty">Nothing collected yet.</div>'}
      </div>`;
  }

  function roundOfArtifact(a) {
    const m = String(a.path || '').match(/\/rounds\/r(\d+)\//);
    return m ? Number(m[1]) : -1;
  }

  function renderFacts() {
    const body = $('#factsBody');
    if (!body) return;
    const top = body.scrollTop;
    body.innerHTML = factsHtml();
    const bar = $('.bar i', body);
    if (bar) bar.style.width = `${bar.dataset.pct || 0}%`;
    if (state.lastRunId === state.currentId) body.scrollTop = top;
    else body.scrollTop = 0;
    $('#factsLbl').textContent = state.detail ? 'Run' : 'No run';
  }

  // ---- chrome ----------------------------------------------------------------------------------

  function renderConn() {
    const dot = $('#connDot');
    const label = $('#connLabel');
    if (!dot || !label) return;
    let text = 'connecting';
    let cls = '';
    if (state.health === false) {
      text = 'offline';
      cls = 'bad';
    } else if (state.health === true) {
      if (!state.currentId) {
        text = 'idle';
      } else if (state.conn === 'open') {
        text = 'live';
        cls = 'ok';
      } else {
        text = 'reconnecting';
        cls = 'warn';
      }
    }
    dot.className = 'dot ' + cls;
    label.textContent = text;
    $('#conn').title = state.health === false
      ? 'The runner on 127.0.0.1:4737 is not answering'
      : state.conn === 'open' ? 'The run journal is streaming live' : 'The event stream is not open';
  }

  function renderChrome() {
    const project = projectPath();
    $('#crumbProject').textContent = project ? projectName(project) : 'Ilvuno';
    const title = state.detail ? runTitle() : state.runs.length ? 'No run selected' : 'No runs yet';
    $('#crumbTitle').textContent = title;
    $('#crumbTitle').title = title;
    $('#settingsVersion').textContent = state.version ? `v${state.version}` : 'Ilvuno';
    document.title = state.detail ? `${runTitle()} — Ilvuno` : 'Ilvuno';

    const offline = state.health === false;
    $('#offlineBanner').hidden = !offline;
    $('#app').classList.toggle('is-offline', offline);
    renderConn();
    applyInert();
  }

  // ---- drawer ----------------------------------------------------------------------------------

  function renderDrawer() {
    const drawer = $('#drawer');
    const index = state.drawerIndex;
    const t = state.detail?.tasks?.[index];
    if (!t) {
      drawer.classList.remove('open');
      $('#drawerScrim').classList.remove('open');
      return;
    }
    $('#drawerTitle').textContent = `#${index + 1} · ${t.thread}`;
    $('#drawerState').innerHTML = t.status ? chip(statusWord(t.status), statusClass(t.status)) : '';

    const arts = (state.detail.artifacts || []).filter((a) => {
      const m = String(a.path || '').match(/\/rounds\/r(\d+)\/([^/]+)\//);
      return m && Number(m[1]) === Number(t.round) && m[2] === t.thread;
    });

    const scope = t.scope && t.scope.length
      ? t.scope.map((p) => p === 'run'
        ? `<div class="dfile">${ic('folder')}<span>Whole run</span></div>`
        : `<div class="dfile" title="${esc(p)}">${ic('folder')}<span>${esc(relPath(p))}</span></div>`).join('')
      : '<div class="dnone">No files declared.</div>';

    const produced = arts.length
      ? arts.map((a) => {
        const fn = String(a.path || '').split('/').pop();
        return `<div class="dfile" title="${esc(a.path)}">${ic('folder')}<span>${esc(fn)}</span><span class="sz">${a.bytes != null ? fmt(a.bytes) + ' b' : '—'}</span></div>`;
      }).join('')
      : '<div class="dnone">No artifact was collected for this task.</div>';

    $('#drawerBody').innerHTML = `
      <div class="dsec">
        <h3>Prompt <span>round ${esc(t.round)}</span></h3>
        <div class="dprompt">${esc(shrinkText(t.prompt))}</div>
      </div>
      <div class="dsec">
        <h3>What the agent said <span>${fmt((t.message || '').length)} chars</span></h3>
        <div class="dmsg" tabindex="0">${esc(shrinkText(t.message || 'No message.'))}</div>
      </div>
      <div class="dsec">
        <h3>Report <span>by ${esc(t.thread)}</span></h3>
        <div class="dreport">
          <div class="ok">${ic('check')} reported</div>
          ${esc(shrinkText(t.result || 'No result line.'))}
        </div>
      </div>
      <div class="dsec">
        <h3>Scope <span>${(t.scope || []).length}</span></h3>
        ${scope}
      </div>
      <div class="dsec">
        <h3>Produced <span>${arts.length}</span></h3>
        ${produced}
      </div>`;
  }

  // ---- settings --------------------------------------------------------------------------------

  function themeSeg(current) {
    const opt = (value, label) => `<label><input type="radio" name="theme" value="${value}" data-setting="theme"${current === value ? ' checked' : ''}><span>${label}</span></label>`;
    return `<div class="seg" role="radiogroup" aria-label="Theme">${opt('dark', 'Dark')}${opt('light', 'Light')}${opt('system', 'System')}</div>`;
  }

  function policySeg(current) {
    const opt = (value, label) => `<label><input type="radio" name="policy" value="${value}" data-setting="message_policy"${current === value ? ' checked' : ''}><span>${label}</span></label>`;
    return `<div class="seg" role="radiogroup" aria-label="Message policy">${opt('never', 'Never')}${opt('round', 'At round end')}${opt('anytime', 'Anytime')}</div>`;
  }

  function settingsHtml(tab) {
    if (tab === 'agents') {
      if (state.agentsLoading && !state.agents) return '<p class="settings-loading">Looking for agent CLIs on the PATH…</p>';
      if (state.agentsError && !state.agents) {
        return `<p class="nr-error" role="alert">${esc(state.agentsError)}</p><button class="btn" data-act="reload-agents">Try again</button>`;
      }
      const agents = state.agents || [];
      return `
        <p class="set-sub">Ilvuno runs the CLIs already set up on your machine. It never holds a model key.</p>
        <div class="set-sec">
          ${agents.map((a) => `
            <div class="agent">
              <div class="agent-top">
                <span class="agent-name">${esc(a.name)}</span>
                <span class="agent-ver">${a.found ? esc(a.version || 'version unknown') : '—'}</span>
                <span class="agent-state"><span class="dot ${a.found ? 'ok' : 'bad'}"></span>${a.found ? 'found' : 'missing'}</span>
              </div>
              <div class="agent-path" title="${esc(a.path || '')}">${esc(a.path || 'not on the PATH of the ilvunod process')}</div>
              <div class="agent-safety"><span class="chip ${a.safety === 'unknown' ? '' : 'safe'}">permissions</span><span>${esc(safetyLine(a.safety))}</span></div>
            </div>`).join('') || '<div class="facts-empty">The API reported no agents.</div>'}
        </div>
        <div class="note">Ilvuno does not sandbox agents; each CLI is bounded by its own permissions.</div>`;
    }
    if (tab === 'defaults') {
      const s = state.settings;
      if (!s) return state.settingsError ? `<p class="nr-error" role="alert">${esc(state.settingsError)}</p>` : '<p class="settings-loading">Reading settings…</p>';
      return `
        ${state.settingsError ? `<p class="nr-error" role="alert">${esc(state.settingsError)}</p>` : ''}
        <p class="set-sub">These apply to new runs. The run on screen keeps its own budget.</p>
        <div class="set-sec">
          <div class="k">Budget</div>
          <div class="form-row">
            <div class="form-label">Rounds<small>The runner refuses to start another round past this number.</small></div>
            <input class="num" type="number" min="1" max="20" value="${esc(s.budget_rounds)}" data-setting="budget_rounds" aria-label="Default rounds">
          </div>
          <div class="form-row">
            <div class="form-label">Tokens<small>A soft stop. The round in flight finishes; no new round starts.</small></div>
            <input class="num" type="text" inputmode="numeric" value="${esc(fmt(s.budget_tokens))}" data-setting="budget_tokens" aria-label="Default tokens">
          </div>
        </div>
        <div class="set-sec">
          <div class="k">Messages</div>
          <div class="form-row">
            <div class="form-label">Message policy<small>When one thread may write to another.</small></div>
            ${policySeg(s.message_policy)}
          </div>
          <div class="form-row">
            <div class="form-label">Human gate<small>Nothing is accepted until you accept the run.</small></div>
            <button class="switch" role="switch" aria-checked="${s.human_gate ? 'true' : 'false'}" aria-label="Human gate" data-act="toggle-gate"></button>
          </div>
        </div>
        <div class="note">These are written to <code>PUT /settings</code>. The run on screen is not changed.</div>`;
    }
    if (tab === 'about') {
      return `
        <p class="set-sub">A local runner for coding agents. One project, a small team of threads, and a record of every handoff.</p>
        <div class="set-sec">
          <div class="kv"><span>Version</span><span class="v">${esc(state.version || '—')}</span></div>
          <div class="kv"><span>Runs on this machine</span><span class="v">${state.runs.length}</span></div>
          <div class="kv"><span>Local API</span><span class="v">127.0.0.1:4737</span></div>
          <div class="kv"><span>Data folder</span><span class="v">set by ILVUNO_HOME for ilvunod</span></div>
        </div>
        <div class="note">The window reads the local API and holds no engine code. Starting and stopping the runner is up to you; this window cannot do it.</div>
        <div class="note">Ilvuno does not sandbox agents; each CLI is bounded by its own permissions.</div>`;
    }
    const project = projectPath();
    const s = state.settings;
    return `
      <p class="set-sub">How the app looks, and where this project lives.</p>
      <div class="set-sec">
        <div class="k">Appearance</div>
        ${state.settingsError ? `<p class="nr-error" role="alert">${esc(state.settingsError)}</p>` : ''}
        ${s ? `<div class="form-row"><div class="form-label">Theme<small>Written to the runner, so it survives a restart.</small></div>${themeSeg(s.theme)}</div>` : '<p class="settings-loading">Reading settings…</p>'}
      </div>
      <div class="set-sec">
        <div class="k">Project folder</div>
        <div class="path-row">
          <div class="path">${esc(project || 'No run selected.')}</div>
        </div>
      </div>
      <div class="set-sec">
        <div class="k">Keyboard</div>
        <div class="kv"><span>Command palette</span><span class="keys"><span class="kbd">⌘K</span></span></div>
        <div class="kv"><span>New run</span><span class="keys"><span class="kbd">⌘N</span></span></div>
        <div class="kv"><span>Close a panel or dialog</span><span class="keys"><span class="kbd">Esc</span></span></div>
      </div>
      <div class="note">Agent detection is on the Agents tab. A real run fails on spawn rather than guessing.</div>`;
  }

  function renderSettings() {
    const tab = state.settingsTab;
    const titles = { general: 'General', agents: 'Agents', defaults: 'Defaults', about: 'About' };
    const main = $('.set-main');
    const top = main ? main.scrollTop : 0;
    $('#settingsTitle').textContent = titles[tab] || 'General';
    $('#settingsBody').innerHTML = settingsHtml(tab);
    $('#settingsBody').setAttribute('aria-labelledby', 'settingsTab-' + (titles[tab] ? tab : 'general'));
    $$('.set-tab').forEach((b) => {
      const on = b.dataset.tab === tab;
      b.classList.toggle('on', on);
      b.setAttribute('aria-selected', String(on));
      b.tabIndex = on ? 0 : -1;
    });
    if (main) main.scrollTop = top;
    state.settingsRoute = routeName() === 'settings' ? tab : '';
    if (tab === 'agents' && !state.agents && !state.agentsError && !state.agentsLoading) loadAgents();
  }

  async function loadAgents() {
    state.agentsLoading = true;
    state.agentsError = '';
    renderSettings();
    try {
      state.agents = await getJson('/agents');
    } catch (err) {
      state.agentsError = err.message;
    } finally {
      state.agentsLoading = false;
      renderSettings();
    }
  }

  async function loadSettings() {
    try {
      state.settings = await getJson('/settings');
      state.settingsError = '';
      applyTheme();
    } catch (err) {
      state.settingsError = err.message;
    }
    renderAll();
  }

  async function saveSettings(patch, { themeToo = false } = {}) {
    const before = state.settings;
    if (themeToo && patch.theme) {
      // Paint first: a theme switch must not wait for a round trip.
      state.settings = { ...(before || {}), ...patch };
      applyTheme();
      renderRail();
      renderSettings();
    }
    try {
      state.settings = await putJson('/settings', patch);
      state.settingsError = '';
      applyTheme();
      renderRail();
      renderChrome();
      if (dialogName() === 'settings') renderSettings();
      showToast('Setting saved.');
    } catch (err) {
      state.settings = before;
      state.settingsError = err.message;
      applyTheme();
      renderRail();
      renderChrome();
      if (dialogName() === 'settings') renderSettings();
      showToast(err.message);
    }
  }

  // ---- palette ---------------------------------------------------------------------------------

  function paletteActions() {
    const status = state.detail?.run?.status;
    const atGate = status === 'reviewed';
    const canResume = status === 'blocked' || status === 'budget_exhausted';
    const out = [{ act: 'new-run', label: 'New run', icon: 'plus', keys: 'new run start', on: true }];
    if (atGate) {
      out.push({ act: 'accept-gate', label: 'Accept gate', icon: 'check', keys: 'accept merge gate done', on: true });
      out.push({ act: 'send-back', label: 'Send back', icon: 'x', keys: 'send back reject gate note', on: true });
    }
    out.push({ act: 'freeze', label: 'Freeze run', icon: 'clock', keys: 'freeze stop pause run', on: false, hint: 'no API route' });
    if (canResume) out.push({ act: 'resume', label: 'Resume run', icon: 'chevron-right', keys: 'resume continue blocked budget', on: true });
    out.push({ act: 'settings', label: 'Settings', icon: 'settings', keys: 'settings preferences agents', on: true });
    out.push({ act: 'toggle-theme', label: `Toggle theme (${resolveTheme(state.settings?.theme || 'dark') === 'dark' ? 'to light' : 'to dark'})`, icon: 'moon', keys: 'theme dark light toggle', on: true });
    return out;
  }

  let palActive = 0;

  function paletteHtml() {
    const runSection = state.runs.length
      ? `<div class="pal-section" data-group="Runs"><div class="pal-group">Runs</div>
          ${state.runs.map((r) => `
            <button class="pal-item" id="pal-run-${esc(r.id)}" data-act="palette-run" data-id="${esc(r.id)}" data-search="${esc(`${r.title || ''} ${statusWord(r.status)} ${projectName(r.project)}`.toLowerCase())}" role="option" aria-selected="false">
              ${ic('folder')}
              <span class="lbl">${esc(r.title || 'Untitled run')}</span>
              <span class="hint">${esc(statusWord(r.status))}</span>
            </button>`).join('')}
        </div>`
      : '';
    const actions = paletteActions().map((a, i) => `
      <button class="pal-item" id="pal-act-${i}" data-act="${a.act}" data-search="${esc(a.label.toLowerCase() + ' ' + a.keys)}" role="option" aria-selected="false"${a.on ? '' : ' aria-disabled="true"'}>
        ${ic(a.icon)}
        <span class="lbl">${esc(a.label)}</span>
        ${a.hint ? `<span class="hint">${esc(a.hint)}</span>` : ''}
      </button>`).join('');
    return `${runSection}<div class="pal-section" data-group="Actions"><div class="pal-group">Actions</div>${actions}</div><div class="pal-empty" id="palEmpty" hidden>No matching runs or actions.</div>`;
  }

  function openPalette() {
    $('#palList').innerHTML = paletteHtml();
    const input = $('#palInput');
    if (input) {
      input.value = '';
      input.removeAttribute('aria-activedescendant');
    }
    filterPalette();
  }

  function allPaletteItems() {
    return $$('.pal-item', $('#palList')).filter((el) => !el.hidden);
  }

  function markPalette() {
    const items = allPaletteItems();
    items.forEach((el, i) => {
      const on = i === palActive;
      el.classList.toggle('on', on);
      el.setAttribute('aria-selected', String(on));
      if (on) {
        $('#palInput')?.setAttribute('aria-activedescendant', el.id || '');
        el.scrollIntoView({ block: 'nearest' });
      }
    });
  }

  function filterPalette() {
    const input = $('#palInput');
    if (!input) return;
    const q = input.value.trim().toLowerCase();
    let count = 0;
    $$('.pal-section', $('#palList')).forEach((sec) => {
      let shown = 0;
      $$('.pal-item', sec).forEach((el) => {
        const hit = !q || String(el.dataset.search || '').includes(q);
        el.hidden = !hit;
        if (hit) {
          shown++;
          count++;
        }
      });
      sec.hidden = shown === 0;
    });
    const empty = $('#palEmpty');
    if (empty) empty.hidden = count > 0;
    const countEl = $('#palCount');
    if (countEl) countEl.textContent = q ? `${count} result${count === 1 ? '' : 's'}` : `${state.runs.length} runs · ${paletteActions().length} actions`;
    palActive = 0;
    markPalette();
  }

  // ---- new run ---------------------------------------------------------------------------------

  function newRunError(message) {
    const el = $('#nr-error');
    if (!el) return;
    el.textContent = message;
    el.hidden = !message;
  }

  async function startRun() {
    if (state.startBusy) return;
    const raw = $('#nr-goal')?.value.trim() || '';
    newRunError('');
    if (!raw) {
      newRunError('Paste a goal JSON first.');
      $('#nr-goal')?.focus();
      return;
    }
    try {
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        newRunError('The goal must be a JSON object, the same shape as a goal file.');
        return;
      }
    } catch (err) {
      newRunError(`The goal is not valid JSON: ${err.message}`);
      return;
    }
    state.startBusy = true;
    const button = $('[data-act="start-run"]');
    if (button) button.disabled = true;
    try {
      const res = await postJson('/runs', raw);
      const id = res?.run_id;
      if (!id) throw new Error('the runner accepted the goal but returned no run id');
      await refreshRuns();
      state.currentId = id;
      state.detail = null;
      state.board = '';
      state.tab = 'board';
      state.drawerIndex = -1;
      state.lastRunId = null;
      state.expanded = new Set();
      setRoute('board');
      connectEvents(id);
      await loadRun(id);
      $('#nr-goal').value = '';
      showToast(`Run started (${res.status || 'working'}).`);
    } catch (err) {
      newRunError(err.message);
    } finally {
      state.startBusy = false;
      const btn = $('[data-act="start-run"]');
      if (btn) btn.disabled = false;
    }
  }

  // ---- actions that talk to the run ------------------------------------------------------------

  function gateInline(message, cls = '') {
    const el = $('#gateInline');
    if (!el) return;
    el.hidden = !message;
    el.textContent = message;
    el.className = `gate-inline ${cls}`.trim();
  }

  async function decideGate(decision, note = '') {
    if (!state.currentId || state.gateBusy) return;
    state.gateBusy = true;
    renderBoard();
    gateInline(decision === 'accept' ? 'Accepting…' : 'Sending back…');
    try {
      const body = decision === 'accept' ? { decision } : { decision, note };
      await postJson(`/runs/${encodeURIComponent(state.currentId)}/gate`, body);
      state.noteDrafts.delete(state.currentId);
      state.gateBusy = false;
      await loadRun(state.currentId, { refresh: true });
      await refreshRuns();
      showToast(decision === 'accept' ? 'Gate accepted.' : 'Sent back to the team.');
    } catch (err) {
      state.gateBusy = false;
      renderBoard();
      gateInline(err.message, 'bad');
      showToast(err.message);
    }
  }

  function sendBackFromPanel() {
    const note = (state.noteDrafts.get(state.currentId) || $('#gate-note')?.value || '').trim();
    if (!note) {
      gateInline('Write a note first. The team needs to know what to change.', 'warn');
      $('#gate-note')?.focus();
      return;
    }
    decideGate('send_back', note);
  }

  async function resumeRun() {
    if (!state.currentId) return;
    try {
      const res = await postJson(`/runs/${encodeURIComponent(state.currentId)}/resume`, {});
      showToast(`Run resumed (${statusWord(res.status)}).`);
      await loadRun(state.currentId, { refresh: true });
      await refreshRuns();
      connectEvents(state.currentId);
    } catch (err) {
      showToast(err.message);
    }
  }

  async function copyRunId() {
    const id = String(state.currentId || '');
    if (!id) return;
    try {
      await navigator.clipboard.writeText(id);
      showToast('Run id copied.');
      return;
    } catch (err) {
      // The async clipboard can be refused; fall back to a selection.
    }
    const ta = document.createElement('textarea');
    ta.value = id;
    ta.setAttribute('readonly', '');
    ta.className = 'copy-fallback';
    document.body.append(ta);
    ta.select();
    let ok = false;
    try {
      ok = document.execCommand('copy');
    } catch (err) {
      ok = false;
    }
    ta.remove();
    showToast(ok ? 'Run id copied.' : 'Copy failed. Select the id instead.');
  }

  function showChecks() {
    const el = $('#checks');
    if (!el) return;
    if (dialogName()) setRoute(state.tab === 'tasks' || state.tab === 'log' ? state.tab : 'board');
    setTimeout(() => {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      el.classList.add('flash');
      setTimeout(() => el.classList.remove('flash'), 1400);
    }, 40);
  }

  async function toggleTheme() {
    const next = resolveTheme(state.settings?.theme || cachedTheme() || 'dark') === 'dark' ? 'light' : 'dark';
    if (state.settings) {
      await saveSettings({ theme: next }, { themeToo: true });
      return;
    }
    // Settings could not be read (offline). Change the window now; the runner is told on the next save.
    try {
      localStorage.setItem('ilvuno-theme', next);
    } catch (err) {
      // No storage, no persistence; the window still changes.
    }
    applyTheme();
    renderRail();
    showToast(next === 'light' ? 'Light theme.' : 'Dark theme.');
  }

  // ---- render all ------------------------------------------------------------------------------

  function focusAfterRender() {
    const name = dialogName();
    if (!name && state.lastDialog) {
      restoreOpener(state.lastDialog);
      state.lastDialog = null;
      state.lastFocusKey = '';
      return;
    }
    if (name) {
      state.lastDialog = name;
      focusDialogOnce(name);
    }
  }

  function captureFocus() {
    const node = document.activeElement;
    if (!node || !node.closest || !node.closest('#app')) return null;
    const act = node.dataset.act;
    if (!act) return null;
    if (act === 'more') return `[data-act="more"][data-target="${node.dataset.target}"]`;
    if (act === 'select-run') return `[data-act="select-run"][data-id="${node.dataset.id}"]`;
    if (act === 'tab') return `[data-act="tab"][data-tab="${node.dataset.tab}"]`;
    if (node.dataset.i !== undefined) return `[data-act="${act}"][data-i="${node.dataset.i}"]`;
    return `[data-act="${act}"]`;
  }

  function restoreFocus(selector) {
    const el = selector && document.querySelector(selector);
    if (!el || el.closest('[inert]')) return;
    setTimeout(() => el.focus({ preventScroll: true }), 0);
  }

  function viewSignature() {
    return JSON.stringify({
      id: state.currentId, detail: state.detail, board: state.board, boardError: state.boardError,
      runs: state.runs, runsError: state.runsError, health: state.health, version: state.version,
      conn: state.conn, tab: state.tab, drawer: state.drawerIndex, settings: state.settings,
      settingsError: state.settingsError, settingsTab: state.settingsTab, agents: state.agents,
      agentsError: state.agentsError, agentsLoading: state.agentsLoading, gateBusy: state.gateBusy,
      route: routeName(),
    });
  }

  let lastSignature = '';

  function renderAll(force = false) {
    const signature = viewSignature();
    if (!force && signature === lastSignature) return;
    lastSignature = signature;
    const focused = captureFocus();

    renderRail();
    renderBoard();
    renderFacts();
    renderChrome();

    const settingsOpen = routeName() === 'settings';
    const paletteOpen = routeName() === 'palette';
    const newRunOpen = routeName() === 'newrun';
    const drawerOpen = routeName() === 'drawer';

    $('#settingsOverlay').classList.toggle('open', settingsOpen);
    if (settingsOpen) {
      if (state.settingsRoute !== state.settingsTab) renderSettings();
    } else {
      state.settingsRoute = '';
    }
    $('#paletteOverlay').classList.toggle('open', paletteOpen);
    $('#newrunOverlay').classList.toggle('open', newRunOpen);
    $('#drawerScrim').classList.toggle('open', drawerOpen);
    $('#drawer').classList.toggle('open', drawerOpen);
    if (drawerOpen) renderDrawer();
    else {
      $('#drawer').classList.remove('open');
      $('#drawerScrim').classList.remove('open');
    }

    applyInert();
    focusAfterRender();
    if (!dialogElement()) restoreFocus(focused);
    state.lastRunId = state.currentId;
  }

  // ---- input and events ------------------------------------------------------------------------

  function selectRun(id) {
    if (!id) return;
    const changing = id !== state.currentId;
    state.currentId = id;
    state.drawerIndex = -1;
    if (changing) {
      state.detail = null;
      state.board = '';
      state.boardError = '';
      state.tab = 'board';
      state.expanded = new Set();
    }
    connectEvents(id);
    renderAll();
    loadRun(id);
  }

  function handleAction(act, target) {
    switch (act) {
      case 'palette':
        state.opener = target;
        openDialog('palette');
        break;
      case 'new-run':
        state.opener = target;
        newRunError('');
        openDialog('newrun');
        break;
      case 'close-newrun':
        closeDialog();
        break;
      case 'settings':
        state.opener = target;
        openDialog('settings');
        break;
      case 'close-settings':
        closeDialog();
        break;
      case 'close-drawer':
        closeDialog();
        break;
      case 'select-run':
        setRoute('board');
        selectRun(target.dataset.id);
        break;
      case 'palette-run':
        setRoute('board');
        selectRun(target.dataset.id);
        break;
      case 'tab':
        state.tab = target.dataset.tab;
        setRoute(state.tab);
        break;
      case 'task':
        state.opener = target;
        state.drawerIndex = Number(target.dataset.i) || 0;
        setRoute('task-' + (state.drawerIndex + 1));
        break;
      case 'settings-tab':
        setRoute('settings-' + target.dataset.tab);
        break;
      case 'toggle-theme':
        if (dialogName() === 'palette') closeDialog();
        toggleTheme();
        break;
      case 'accept-gate':
        if (dialogName() === 'palette') closeDialog();
        decideGate('accept');
        break;
      case 'send-back':
        if (dialogName() === 'palette') closeDialog();
        sendBackFromPanel();
        break;
      case 'resume':
        if (dialogName() === 'palette') closeDialog();
        resumeRun();
        break;
      case 'freeze':
        showToast('The local API has no freeze route; nothing was changed.');
        break;
      case 'toggle-gate': {
        if (!state.settings) return;
        const next = !state.settings.human_gate;
        target.setAttribute('aria-checked', String(next));
        saveSettings({ human_gate: next });
        break;
      }
      case 'copy-id':
        copyRunId();
        break;
      case 'checks':
        showChecks();
        break;
      case 'more': {
        const body = document.getElementById(target.dataset.target);
        if (!body) return;
        const open = target.getAttribute('aria-expanded') === 'true';
        body.classList.toggle('clamp', open);
        target.setAttribute('aria-expanded', String(!open));
        target.textContent = open ? 'more' : 'less';
        if (open) state.expanded.delete(body.id);
        else state.expanded.add(body.id);
        break;
      }
      case 'start-run':
        startRun();
        break;
      case 'reload-agents':
        state.agentsError = '';
        loadAgents();
        break;
      case 'retry':
        retry();
        break;
      default:
        break;
    }
  }

  let retrying = false;
  async function retry() {
    if (retrying) return;
    retrying = true;
    const button = $('.offline-banner .btn');
    if (button) {
      button.disabled = true;
      button.textContent = 'Retrying…';
    }
    await poll({ initial: !state.detail });
    retrying = false;
    const btn = $('.offline-banner .btn');
    if (btn) {
      btn.disabled = false;
      btn.textContent = 'Retry';
    }
    if (state.health === false) showToast('Still not reachable. Start the runner, then retry.');
  }

  document.addEventListener('click', (e) => {
    const target = e.target.closest('[data-act]');
    if (!target) return;
    if (target.getAttribute('aria-disabled') === 'true') return;
    handleAction(target.dataset.act, target);
  });

  document.addEventListener('input', (e) => {
    if (e.target.id === 'palInput') filterPalette();
    if (e.target.id === 'gate-note' && state.currentId) state.noteDrafts.set(state.currentId, e.target.value);
  });

  document.addEventListener('change', (e) => {
    const input = e.target;
    const setting = input.dataset?.setting;
    if (!setting) return;
    if (setting === 'theme') {
      saveSettings({ theme: input.value }, { themeToo: true });
      return;
    }
    if (setting === 'message_policy') {
      saveSettings({ message_policy: input.value });
      return;
    }
    if (setting === 'budget_rounds' || setting === 'budget_tokens') {
      const digits = String(input.value).replace(/[^0-9]/g, '');
      if (!digits) {
        renderSettings();
        return;
      }
      const value = Number(digits);
      saveSettings({ [setting]: value });
    }
  });

  document.addEventListener('submit', (e) => e.preventDefault());

  $('#drawerScrim').addEventListener('click', closeDialog);
  $('#settingsOverlay').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeDialog();
  });
  $('#paletteOverlay').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeDialog();
  });
  $('#newrunOverlay').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeDialog();
  });

  $('#palInput').addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const items = allPaletteItems();
      if (!items.length) return;
      palActive = e.key === 'ArrowDown' ? (palActive + 1) % items.length : (palActive - 1 + items.length) % items.length;
      markPalette();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const items = allPaletteItems();
      const item = items[palActive];
      if (item && item.getAttribute('aria-disabled') !== 'true') item.click();
    }
  });

  document.addEventListener('keydown', (e) => {
    const key = String(e.key).toLowerCase();
    const dialog = dialogElement();

    if ((e.metaKey || e.ctrlKey) && key === 'k') {
      e.preventDefault();
      if (dialogName() === 'palette') closeDialog();
      else openDialog('palette');
      return;
    }
    if ((e.metaKey || e.ctrlKey) && key === 'n') {
      e.preventDefault();
      if (dialogName() === 'newrun') closeDialog();
      else openDialog('newrun');
      return;
    }

    if (dialog && e.key === 'Tab') {
      trapTab(e, dialog);
      return;
    }

    const setTab = e.target.closest ? e.target.closest('.set-tab') : null;
    if (setTab && ['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) {
      e.preventDefault();
      const tabs = $$('.set-tab');
      const i = tabs.indexOf(setTab);
      const j = e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1
        : e.key === 'ArrowDown' ? (i + 1) % tabs.length : (i - 1 + tabs.length) % tabs.length;
      tabs[j].focus();
      setRoute('settings-' + tabs[j].dataset.tab);
      return;
    }

    const tabBtn = e.target.closest ? e.target.closest('.tab') : null;
    if (tabBtn && ['ArrowRight', 'ArrowLeft', 'Home', 'End'].includes(e.key)) {
      e.preventDefault();
      const tabs = $$('.tab');
      const i = tabs.indexOf(tabBtn);
      const j = e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1
        : e.key === 'ArrowRight' ? (i + 1) % tabs.length : (i - 1 + tabs.length) % tabs.length;
      tabs[j].focus();
      state.tab = tabs[j].dataset.tab;
      setRoute(state.tab);
      return;
    }

    if (e.key === 'Escape' && routeName() !== 'board') {
      e.preventDefault();
      closeDialog();
    }
  });

  window.addEventListener('hashchange', applyRoute);

  // ---- init ------------------------------------------------------------------------------------

  renderAll();
  loadSettings();
  poll({ initial: true });
  setInterval(() => poll(), LIST_EVERY_MS);
  requestAnimationFrame(() => requestAnimationFrame(() => document.documentElement.classList.remove('boot')));
})();
