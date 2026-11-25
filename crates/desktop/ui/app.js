// The Ilvuno window. It reads the local API (docs/api.md) and renders what it answers. Nothing on
// screen is invented: every run, title, number, agent and sentence comes from a response.
//
// Layout, from research/13-ui-anatomy.md §5: threads on the left, the selected thread's conversation in
// the centre, the run's shared work on the right. A run is context; a thread is what you work in.
//
// The port is a constant, not a setting, because the Tauri CSP (tauri.conf.json) names the same origin.
// Change both together if the daemon moves. ILVUNO_PORT only changes the daemon side.
(() => {
  'use strict';

  const API = 'http://127.0.0.1:4737';
  const LIST_EVERY_MS = 5000;
  const HEALTH_TIMEOUT_MS = 2000;
  const RELOAD_AFTER_EVENT_MS = 300;
  const GROUP_ORDER = [['needs', 'Needs you'], ['working', 'Working'], ['done', 'Done']];
  const SAFETY_LABEL = { asks_before_each_tool: 'Asks first', works_in_folder_asks_outside: 'Folder only' };

  const $ = (sel, root = document) => root.querySelector(sel);
  const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));
  const esc = (v) => String(v ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

  // Agents write markdown. We render a small, safe subset — bold, inline code, bullets, headings, code
  // fences — because the alternative is a sentence with "**Task queued:**" printed inside it.
  function md(text) {
    const inline = (s) => s
      .replace(/`([^`\n]+)`/g, '<code>$1</code>')
      .replace(/\*\*([^*\n]+)\*\*/g, '<strong>$1</strong>');
    let out = '';
    let list = false;
    let fence = false;
    for (const raw of String(text ?? '').split('\n')) {
      const line = raw.trimEnd();
      if (/^\s*```/.test(line)) {
        out += fence ? '</pre>' : '<pre>';
        fence = !fence;
        continue;
      }
      if (fence) {
        out += esc(raw) + '\n';
        continue;
      }
      const bullet = /^\s*[-*]\s+(.*)$/.exec(line);
      if (bullet) {
        if (!list) {
          out += '<ul>';
          list = true;
        }
        out += `<li>${inline(esc(bullet[1]))}</li>`;
        continue;
      }
      if (list) {
        out += '</ul>';
        list = false;
      }
      if (!line.trim()) {
        out += '<p class="gap"></p>';
        continue;
      }
      const head = /^#{1,4}\s+(.*)$/.exec(line);
      out += head ? `<p class="md-h">${inline(esc(head[1]))}</p>` : `<p>${inline(esc(line))}</p>`;
    }
    if (list) out += '</ul>';
    if (fence) out += '</pre>';
    return out;
  }
  const fmt = (n) => new Intl.NumberFormat('en-US').format(Number(n) || 0);
  // One format for a count that gets large: 5.4k, 197k. A byte count is a size, so it gets its own.
  const compact = (n) => {
    const v = Number(n) || 0;
    if (v < 1000) return fmt(v);
    if (v < 100000) return (v / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
    return Math.round(v / 1000) + 'k';
  };
  // Cut prose at a word, so a cut line ends on a word and says it was cut.
  const clip = (s, n) => {
    const t = String(s || '').replace(/\s+/g, ' ').trim();
    if (t.length <= n) return t;
    const cut = t.slice(0, n);
    const i = cut.lastIndexOf(' ');
    return (i > n * 0.6 ? cut.slice(0, i) : cut) + '…';
  };
  const bytes = (n) => {
    const v = Number(n) || 0;
    if (v < 1000) return `${fmt(v)} B`;
    if (v < 1e6) return (v / 1000).toFixed(1).replace(/\.0$/, '') + ' kB';
    return (v / 1e6).toFixed(1).replace(/\.0$/, '') + ' MB';
  };
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
    appVersion: null,
    conn: 'connecting', // open | reconnecting | offline | connecting
    runs: [],
    runsError: '',
    currentId: null,
    detail: null,
    activityRows: [],
    activityRunId: null,
    activityCursor: 0,
    activityLoaded: false,
    activityError: '',
    activityExpanded: new Set(),
    threadDiffs: new Map(), // IlvunoDiff.cacheKey → { patch, stat, truncated } or { error }
    parsedDiffs: new Map(), // same key → parsed files, so a file list is not re-parsed on every render
    diffSel: null, // { thread, path } of the file open in the Diff tab
    board: '',
    boardError: '',
    view: 'thread', // thread | board
    thread: null, // the selected thread's name
    rightOpen: false,
    settings: null,
    settingsError: '',
    agents: null,
    agentsError: '',
    agentsLoading: false,
    settingsTab: 'general',
    stopping: false,
    agentMenu: false,
    settingsRoute: '',
    drawerIndex: -1,
    expanded: new Set(),
    noteDrafts: new Map(),
    drafts: new Map(), // thread name → unsent composer text
    sendError: null, // { thread, message }
    sending: false,
    opener: null,
    lastDialog: null,
    lastFocusKey: '',
    lastRunId: null,
    lastThreadKey: '',
    toastTimer: 0,
    pollBusy: false,
    gateBusy: false,
    startBusy: false,
    es: null,
    esRunId: null,
    reloadTimer: 0,
    loadSeq: 0,
  };

  async function loadAppVersion() {
    const invoke = window.__TAURI__?.core?.invoke;
    if (typeof invoke !== 'function') return;
    try {
      state.appVersion = await invoke('app_version');
      if (state.settingsRoute === 'about') renderSettings();
    } catch (err) {
      state.appVersion = null;
    }
  }

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

  const TURN_ROW_KINDS = [
    'tool.call', 'permission.decided', 'dispatch.failed',
    'dispatch.queued', 'dispatch.started', 'dispatch.reported',
  ];
  const TOOL_ROW_KINDS = ['tool.call', 'permission.decided', 'dispatch.failed'];

  async function fetchActivity(id, after) {
    const rows = [];
    let cursor = after;
    for (;;) {
      const page = await getJson(`/runs/${encodeURIComponent(id)}/journal?after=${cursor}&kinds=${encodeURIComponent(TURN_ROW_KINDS.join(','))}&limit=1000`);
      if (!Array.isArray(page)) break;
      rows.push(...page);
      if (page.length < 1000) break;
      const next = Number(page[page.length - 1]?.seq);
      if (!Number.isFinite(next) || next <= cursor) break;
      cursor = next;
    }
    return rows;
  }

  function mergeActivityRows(id, rows) {
    if (state.activityRunId !== id || !Array.isArray(rows)) return [];
    const bySeq = new Map(state.activityRows.map((row) => [Number(row.seq), row]));
    const added = [];
    for (const row of rows) {
      const seq = Number(row?.seq);
      if (!Number.isFinite(seq) || bySeq.has(seq)) continue;
      bySeq.set(seq, row);
      added.push(row);
      state.activityCursor = Math.max(state.activityCursor, seq);
    }
    if (added.length) state.activityRows = Array.from(bySeq.values()).sort((a, b) => Number(a.seq) - Number(b.seq));
    return added;
  }

  async function catchUpActivity(id) {
    if (state.activityRunId !== id) {
      state.activityRunId = id;
      state.activityRows = [];
      state.activityCursor = 0;
      state.activityLoaded = false;
      state.activityError = '';
      state.activityExpanded.clear();
    }
    try {
      const rows = await fetchActivity(id, state.activityCursor);
      if (state.activityRunId !== id) return;
      const added = mergeActivityRows(id, rows);
      state.activityError = '';
      if (added.length) activityRowsChanged(id, added);
    } catch (err) {
      if (state.activityRunId === id) state.activityError = err.message;
    }
  }

  function activityRowsChanged(id, rows) {
    if (id !== state.currentId || state.view !== 'thread') return;
    const thread = currentThread();
    if (!thread) return;
    const matchesOpenThread = rows.some((row) => row.thread === thread.name &&
      thread.tasks.some((task) => Number(task.round) === Number(row.round)));
    if (matchesOpenThread) renderView();
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
      state.thread = null;
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
    if (state.activityRunId !== id) {
      state.activityRunId = id;
      state.activityRows = [];
      state.activityCursor = 0;
      state.activityLoaded = false;
      state.activityError = '';
      state.activityExpanded.clear();
    }
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
        state.thread = null;
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
    if (!state.activityLoaded) {
      try {
        const rows = await fetchActivity(id, 0);
        if (seq !== state.loadSeq || state.currentId !== id) return;
        mergeActivityRows(id, rows);
        state.activityLoaded = true;
        state.activityError = '';
      } catch (err) {
        state.activityError = err.message;
      }
    }
    if (seq !== state.loadSeq || state.currentId !== id) return;
    state.detail = detail;
    state.board = board?.markdown || '';
    pickThread();
    renderAll();
    await loadThreadDiffs(id, detail, seq);
  }

  // A thread's patch describes one commit. The cache is keyed by that commit (IlvunoDiff.cacheKey), so
  // a new round fetches a new patch instead of showing the last round's. Runs with no checkout skip this.
  async function loadThreadDiffs(id, detail, seq) {
    // A failed read is not kept: the next refresh asks again, so a runner hiccup does not hide the patch.
    const missing = (detail.threads || []).filter((thread) => {
      if (!thread.has_checkout) return false;
      const cached = state.threadDiffs.get(IlvunoDiff.cacheKey(id, thread));
      return !cached || !!cached.error;
    });
    if (!missing.length) return;
    let changed = false;
    await Promise.all(missing.map(async (thread) => {
      const key = IlvunoDiff.cacheKey(id, thread);
      let value;
      try {
        value = await getJson(`/runs/${encodeURIComponent(id)}/threads/${encodeURIComponent(thread.name)}/diff`);
      } catch (err) {
        value = { error: err.message };
      }
      const before = state.threadDiffs.get(key);
      if (!before || before.error !== value.error || before.patch !== value.patch) changed = true;
      state.threadDiffs.set(key, value);
    }));
    // Re-render only when something new arrived. A diff that keeps failing must not rebuild the view on
    // every refresh, because that would clear a text selection in the patch.
    if (changed && seq === state.loadSeq && state.currentId === id) renderAll(true);
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
      renderChrome();
      scheduleRefresh();
      catchUpActivity(id);
    };
    es.onerror = () => {
      state.conn = state.health === false ? 'offline' : 'reconnecting';
      renderChrome();
    };
    es.addEventListener('journal', (event) => {
      let kind = 'unknown';
      let row = null;
      try {
        row = JSON.parse(event.data || '{}');
        if (typeof row.kind === 'string') kind = row.kind;
      } catch (err) {
        kind = 'unknown';
      }
      if (TURN_ROW_KINDS.includes(kind)) {
        const hasActivity = Number.isFinite(Number(row?.seq)) && Number.isInteger(row?.round) &&
          typeof row?.thread === 'string' && row?.payload && typeof row.payload === 'object';
        if (hasActivity) {
          const added = mergeActivityRows(id, [row]);
          if (added.length) activityRowsChanged(id, added);
        } else {
          catchUpActivity(id);
        }
      }
      if (kind !== 'tool.call' && kind !== 'permission.decided') scheduleRefresh();
    });
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

  const enc = (v) => encodeURIComponent(String(v));

  function routeName() {
    const h = hashText();
    if (h.startsWith('settings')) return 'settings';
    if (h === 'palette' || h === 'newrun' || h === 'board' || h === 'diff') return h;
    if (h === 'drawer' || /^task-\d+$/.test(h)) return 'drawer';
    if (h.startsWith('t/')) return 'thread';
    return 'thread';
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
      case 'palette': return $('#paletteModal');
      case 'settings': return $('#settingsModal');
      case 'newrun': return $('#newrunModal');
      case 'drawer': return $('#drawer');
      default: return null;
    }
  }

  function homeRoute() {
    if (state.view === 'board' || state.view === 'diff') return state.view;
    return state.thread ? 't/' + state.thread : 'thread';
  }

  function setRoute(name) {
    // The thread view with no thread chosen is the home route, so it clears the hash.
    const clear = !name || name === 'thread';
    const url = clear ? location.pathname + location.search : '#' + name;
    try {
      history.replaceState(null, '', url);
    } catch (err) {
      location.hash = clear ? '' : name;
    }
    applyRoute();
  }

  function applyRoute() {
    const name = routeName();
    if (name === 'board' || name === 'diff') {
      state.view = name;
    } else if (name === 'thread') {
      state.view = 'thread';
      const h = hashText();
      if (h.startsWith('t/')) state.thread = h.slice(2);
    }
    if (name === 'drawer') {
      state.view = 'thread';
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
    if (name === 'settings') {
      // Settings come from the runner. Opening the panel is a fresh look, not a verdict on an edit the
      // person already walked away from — a stale error here reads as "settings are broken".
      state.settingsError = '';
      loadSettings();
    }
    setRoute(name);
  }

  function closeDialog() {
    setRoute(homeRoute());
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

  // The rail has room for two folder names, not the whole path; the title attribute keeps the rest.
  function tailPath(path, keep = 2) {
    const raw = String(path || '');
    const parts = raw.split('/').filter(Boolean);
    // Short paths keep their leading slash: "tmp/stop2" would read as a relative path.
    if (parts.length <= keep) return raw.startsWith('/') ? raw : parts.join('/');
    return '…/' + parts.slice(-keep).join('/');
  }

  // Paths are shown relative to the repo root, the same way the export does it.
  function rootPath() {
    const project = projectPath();
    return project ? project.replace(/\/[^/]+$/, '') : '';
  }

  function relPath(p) {
    let s = String(p || '');
    const root = rootPath();
    if (root && s.startsWith(root + '/')) s = s.slice(root.length + 1);
    s = s.replace(/^.*\/runs\/[0-9a-f-]{36}\//, 'run/');
    s = s.replace(/^.*\/worktrees\/[0-9a-f]{8}\//, 'worktree/');
    return s;
  }

  function shrinkText(text) {
    let s = String(text ?? '');
    const root = rootPath();
    if (root) s = s.split(root + '/').join('');
    s = s.replace(/\/(?:[\w.\-]+\/)*runs\/([0-9a-f\-]{36})\//g, 'run/');
    return s;
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

  // `working` means the drive has not stopped. It can still be doing nothing: the round is closed and no
  // task is queued. Saying "working" there is true on paper and false on screen, so the window says the
  // thing a person can check: nothing is running, and a message starts the next round.
  function runIsWaiting() {
    const d = state.detail;
    if (!d || d.run?.status !== 'working') return false;
    const running = (d.tasks || []).some((t) => t.status === 'running');
    const open = (d.rounds || []).some((r) => r.status === 'open');
    return !running && !open;
  }

  function statusWord(status) {
    return String(status || 'unknown').replace(/_/g, ' ');
  }

  function statusClass(status) {
    if (['accepted', 'merged', 'pass', 'pass_with_notes', 'reported', 'ok'].includes(status)) return 'ok';
    if (['blocked', 'budget_exhausted', 'no_progress', 'failed', 'block'].includes(status)) return 'bad';
    if (['reviewed', 'human_paused', 'queued', 'running'].includes(status)) return 'warn';
    return '';
  }

  function statusChip(status) {
    return chip(statusWord(status), statusClass(status));
  }

  // A CLI prints its own version string: "2.1.331 (Claude Code)", "codex-cli 0.172.0", "opencode v2.3.2".
  // The number is the part that matters, so take it and drop the rest.
  function agentVersion(a) {
    const raw = String(a.version || '').trim();
    const number = raw.match(/\d+(?:\.\d+)+/);
    if (number) return number[0];
    return raw.replace(new RegExp(String(a.name || ''), 'ig'), '').trim() || raw || 'version unknown';
  }

  // A thread's agent is a provider id from the goal (`claude-main`, `codex-main`). We match it against
  // the CLIs the API found, so the safety badge is the CLI's own answer where we can get one.
  function safetyOf(agent) {
    const id = String(agent || '').toLowerCase();
    const found = (state.agents || []).filter((a) => a.safety && a.safety !== 'unknown');
    const hit = found.find((a) => id.includes(a.id.toLowerCase()));
    if (hit) return hit.safety;
    if (/claude/.test(id)) return 'asks_before_each_tool';
    if (/codex|pi|opencode|agy|antigravity|cursor/.test(id)) return 'works_in_folder_asks_outside';
    return 'unknown';
  }

  function safetyChip(agent) {
    const safety = safetyOf(agent);
    const label = SAFETY_LABEL[safety] || 'Unknown';
    const note = safety === 'asks_before_each_tool'
      ? 'This agent asks before every tool it uses.'
      : safety === 'works_in_folder_asks_outside'
        ? 'This agent writes inside its folder and asks before anything outside it.'
        : 'This agent has not declared how it asks for permission.';
    return `<span class="chip ${safety === 'unknown' ? '' : 'safe'}" title="${esc(note)}">${esc(label)}</span>`;
  }

  function safetyLine(safety) {
    if (safety === 'asks_before_each_tool') return 'Asks before every tool';
    if (safety === 'works_in_folder_asks_outside') return 'Writes inside its folder, asks before anything outside';
    return 'Permissions unknown';
  }

  // ---- threads: derived from run.threads plus the tasks that ran on them -----------------------

  function runThreads() {
    const d = state.detail;
    if (!d) return [];
    const byName = new Map();
    for (const t of d.threads || []) {
      byName.set(t.name, {
        name: t.name,
        agent: t.agent || 'unknown',
        role: t.role || 'worker',
        status: t.status || 'idle',
        standing: t.prompt || '',
        branch: t.branch || '',
        commit: t.commit || '',
        stat: t.stat || '',
        hasCheckout: !!t.has_checkout,
        tasks: [],
      });
    }
    (d.tasks || []).forEach((task, i) => {
      const name = task.thread || 'unknown';
      if (!byName.has(name)) {
        byName.set(name, { name, agent: 'unknown', role: 'worker', status: 'idle', standing: '', tasks: [] });
      }
      byName.get(name).tasks.push({ ...task, i, kind: task.kind === 'message' ? 'message' : 'task' });
    });
    return Array.from(byName.values());
  }

  function lastTask(thread) {
    return thread?.tasks?.length ? thread.tasks[thread.tasks.length - 1] : null;
  }

  function currentThread() {
    if (!state.detail || state.view !== 'thread') return null;
    const threads = runThreads();
    return threads.find((t) => t.name === state.thread) || threads[0] || null;
  }

  // An open question is a `question` ledger entry by this thread with no `answer` after it.
  function openQuestion(name) {
    const ledger = (state.detail?.ledger || []).slice().sort((a, b) => Number(a.id) - Number(b.id));
    let last = null;
    for (const entry of ledger) {
      if (entry.author !== name) continue;
      if (entry.kind === 'question') last = entry;
      if (entry.kind === 'answer') last = null;
    }
    return last;
  }

  // Needs you: a failed task, or a question it wrote that nobody answered. Working: a task queued or
  // running. Done: everything else, and it stays visible until the run is gone (orca's rule).
  function threadGroup(thread) {
    const failed = thread.tasks.some((t) => t.status === 'failed');
    if (failed || openQuestion(thread.name)) return 'needs';
    if (thread.tasks.some((t) => t.status === 'queued' || t.status === 'running')) return 'working';
    return 'done';
  }

  function groupedThreads() {
    const groups = { needs: [], working: [], done: [] };
    for (const thread of runThreads()) groups[threadGroup(thread)].push(thread);
    return groups;
  }

  function pickThread() {
    const threads = runThreads();
    if (!threads.length) {
      state.thread = null;
      return;
    }
    if (threads.some((t) => t.name === state.thread)) return;
    const groups = groupedThreads();
    const first = groups.needs[0] || groups.working[0] || groups.done[0] || threads[0];
    state.thread = first.name;
  }

  function threadDot(thread) {
    const group = threadGroup(thread);
    return group === 'needs' ? 'warn' : group === 'working' ? 'ok pulse' : '';
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
    setInert($('.rail'), offline || !layout.railOpen);
    setInert($('.facts'), offline || app.dataset.right !== 'open');
    setInert($('#scroll'), offline);
    setInert($('#composer'), offline);
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
      const overlay = modal.closest('.overlay') || modal;
      if (!document.contains(modal) || !overlay.classList.contains('open')) return;
      if (name === 'palette') {
        $('#palInput')?.focus({ preventScroll: true });
      } else if (name === 'drawer') {
        modal.focus({ preventScroll: true });
      } else if (name === 'newrun') {
        // A goal usually starts as a file on this machine, so the path is where a person types first.
        ($('#nr-path')?.value.trim() ? $('#nr-goal') : $('#nr-path'))?.focus({ preventScroll: true });
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
        ? $(`.ask-card[data-i="${state.drawerIndex}"]`)
        : closedName === 'palette'
          ? $('[data-act="palette"]')
          : closedName === 'settings'
            ? $('[data-act="settings"]')
            : $('.rail-switch .rsw-select');
    }
    if (target) setTimeout(() => target.focus({ preventScroll: true }), 0);
  }

  // ---- left rail -------------------------------------------------------------------------------

  function renderRunSelect() {
    const select = $('#runSelect');
    if (!select) return;
    const options = state.runs.map((run) => {
      const title = shortTitle(run.title || 'Untitled run');
      return `<option value="${esc(run.id)}">${esc(title)} — ${esc(statusWord(run.status))}</option>`;
    }).join('');
    const sig = options + '|' + (state.currentId || '');
    if (select.dataset.sig !== sig) {
      select.innerHTML = `<option value="__new">＋ New run…</option>${options || '<option value="" disabled>No runs yet</option>'}`;
      select.dataset.sig = sig;
    }
    select.value = state.runs.length ? (state.currentId || '') : '__new';
  }

  function shortTitle(title) {
    const s = String(title || '');
    return s.length > 46 ? s.slice(0, 45) + '…' : s;
  }

  function threadRow(thread) {
    const on = thread.name === state.thread && state.view === 'thread';
    const task = lastTask(thread);
    const text = task ? clip(task.prompt, 58) : 'No task yet';
    return `
      <button class="trow${on ? ' on' : ''}" data-act="thread" data-name="${esc(thread.name)}"${on ? ' aria-current="true"' : ''}>
        <span class="dot ${threadDot(thread)}"></span>
        <span class="trow-name">${esc(thread.name)}</span>
        <span class="trow-agent">${esc(thread.agent)}</span>
        <span class="trow-task">${esc(text)}</span>
      </button>`;
  }

  function gateRow() {
    if (state.detail?.run?.status !== 'reviewed') return '';
    return `
      <button class="gate-row" data-act="focus-gate">
        ${ic('check')}<span>Gate open · decide</span><span class="go">review</span>
      </button>`;
  }

  function renderRail() {
    const body = $('#railBody');
    const foot = $('#railFoot');
    if (!body || !foot) return;

    const project = projectPath();
    $('#railProject').textContent = project ? projectName(project) : (state.runs.length ? 'all projects' : 'no project');
    $('#railProject').title = project ? shortProject(project) : '';
    renderRunSelect();

    let inner = gateRow();
    if (!state.detail) {
      inner += state.health === false
        ? '<p class="rail-empty">The runner is not answering.</p>'
        : state.runs.length
          ? '<p class="rail-loading">Reading the run…</p>'
          : '<p class="rail-empty">No runs yet.</p>';
    } else {
      const groups = groupedThreads();
      inner += GROUP_ORDER.map(([key, label]) => {
        const threads = groups[key];
        if (!threads.length) return '';
        return `
          <div class="tgroup">
            <div class="tgroup-head"><span>${label}</span><span class="n">${threads.length}</span></div>
            ${threads.map(threadRow).join('')}
          </div>`;
      }).join('');
      if (!runThreads().length) inner += '<p class="rail-empty">This run has no threads. Its goal names none.</p>';
      inner += `
        <button class="rail-newthread" disabled aria-describedby="newThreadWhy"
                title="Threads come from the goal file. Start a new run with the threads you want.">
          ${ic('plus', 'ic sm')}New thread
        </button>
        <p class="rail-empty" id="newThreadWhy" style="margin-top:4px">Threads come from the goal file.</p>`;
    }
    if (state.runsError) inner += `<p class="rail-error" role="alert">${esc(state.runsError)}</p>`;
    body.innerHTML = inner;
    body.hidden = false;

    const effective = resolveTheme(state.settings?.theme || cachedTheme() || 'dark');
    foot.innerHTML = `
      <button class="iconbtn" data-act="settings" aria-label="Settings" title="Settings">${ic('settings')}</button>
      <button class="theme-btn" data-act="toggle-theme" aria-label="Switch to ${effective === 'dark' ? 'light' : 'dark'} theme" title="Toggle theme">
        ${ic('moon')}<span class="theme-label">${effective === 'dark' ? 'Dark' : 'Light'}</span>
      </button>`;
  }

  // ---- top bar ---------------------------------------------------------------------------------

  function runMetaHtml() {
    const d = state.detail;
    if (!d) return '';
    const run = d.run || {};
    const budget = d.goal?.budget || {};
    const atGate = run.status === 'reviewed';
    return [
      statusChip(runIsWaiting() ? 'waiting' : run.status),
      chip(`${roundsUsed()} of ${budget.rounds ?? '—'} rounds · ${compact(run.tokens_used)} of ${budget.tokens ? compact(budget.tokens) : '—'} tokens`, 'num'),
      atGate ? `<button class="meta-btn" data-act="focus-gate">${ic('check', 'ic sm')}Gate open</button>` : '',
      // One click, no dialog: nothing is merged, and Resume sits in the stop banner afterwards.
      run.status === 'working'
        ? `<button class="meta-btn stop" data-act="stop-run" ${state.stopping ? 'aria-disabled="true"' : ''}
             title="The turn in flight finishes; no new task or round starts">${ic('stop', 'ic sm')}${state.stopping ? 'Stopping' : 'Stop'}</button>`
        : '',
    ].join('');
  }

  // ---- centre: thread view ---------------------------------------------------------------------

  // A scope chip is narrow. A long path loses its end in the box, and the end is the part a reader
  // needs (the file). Keep the last folders, put the full path in the title.
  function chipPath(p) {
    const s = relPath(p);
    if (s.length <= 30) return s;
    return '…/' + s.split('/').slice(-3).join('/');
  }

  function scopeChips(scope) {
    if (!scope || !scope.length) return '';
    const items = scope.map((p) => p === 'run'
      ? '<span class="fchip run" title="the whole run">whole run</span>'
      : `<span class="fchip" title="${esc(p)}">${esc(chipPath(p))}</span>`).join('');
    return `<div class="led-scope">${items}</div>`;
  }

  // An artifact is filed under a thread and a round, not under a task. Two tasks on one thread in one
  // round both name the same file, so the files belong to the last of them; earlier turns show none.
  function artifactsFor(task) {
    const lastInRound = (state.detail?.tasks || []).reduce((max, t) => (
      t.thread === task.thread && Number(t.round) === Number(task.round) ? Math.max(max, (state.detail?.tasks || []).indexOf(t)) : max
    ), -1);
    if (task.i !== lastInRound) return [];
    return (state.detail?.artifacts || []).filter((a) => {
      const m = String(a.path || '').match(/\/rounds\/r(\d+)\/([^/]+)\//);
      return m && Number(m[1]) === Number(task.round) && m[2] === task.thread;
    });
  }

  function askHtml(task) {
    const prompt = shrinkText(task.prompt);
    if (task.kind === 'message') {
      return `
        <div class="ask">
          <div class="ask-card bub">
            <div class="ask-meta">${ic('message', 'ic sm')}you · message</div>
            <div class="ask-text">${esc(prompt)}</div>
          </div>
        </div>`;
    }
    const scope = (task.scope || []).length ? ` · ${task.scope.length === 1 && task.scope[0] === 'run' ? 'whole run' : `${task.scope.length} ${task.scope.length === 1 ? 'file' : 'files'}`}` : '';
    return `
      <div class="ask">
        <button class="ask-card" data-act="task" data-i="${task.i}" aria-label="Open task ${task.i + 1} on ${esc(task.thread)}">
          <div class="ask-meta">round ${esc(task.round ?? '—')} · by ${esc(task.created_by || 'runner')}${esc(scope)}</div>
          <div class="ask-text">${esc(prompt)}</div>
        </button>
      </div>`;
  }

  function activityRowsForTask(task) {
    const threadRows = state.activityRows.filter((row) => row.thread === task.thread && Number(row.round) === Number(task.round))
      .sort((a, b) => Number(a.seq) - Number(b.seq));
    const siblings = (state.detail?.tasks || []).map((row, i) => ({ ...row, i }))
      .filter((row) => row.thread === task.thread && Number(row.round) === Number(task.round));
    const taskIndex = siblings.findIndex((row) => row.i === Number(task.i));
    if (taskIndex < 0) return [];
    const queued = threadRows.filter((row) => row.kind === 'dispatch.queued');
    const dispatchId = queued[taskIndex]?.payload?.dispatch_id;
    if (typeof dispatchId !== 'string') return [];

    const started = threadRows.find((row) => row.kind === 'dispatch.started' && row.payload?.dispatch_id === dispatchId);
    if (!started) return threadRows.filter((row) => row.kind === 'dispatch.failed' && row.payload?.dispatch_id === dispatchId);
    const ended = threadRows.find((row) => Number(row.seq) > Number(started.seq) &&
      ['dispatch.reported', 'dispatch.failed'].includes(row.kind) && row.payload?.dispatch_id === dispatchId);
    return threadRows.filter((row) => TOOL_ROW_KINDS.includes(row.kind) && Number(row.seq) > Number(started.seq) &&
      (!ended || Number(row.seq) <= Number(ended.seq)));
  }

  function activityTarget(tool, input) {
    let value = input;
    if (typeof value === 'string') {
      try { value = JSON.parse(value); } catch (err) { return ''; }
    }
    if (!value || typeof value !== 'object' || Array.isArray(value)) return '';
    const keys = /^bash$/i.test(tool)
      ? ['command']
      : /^(read|write|edit|multiedit)$/i.test(tool)
        ? ['file_path', 'path']
        : ['file_path', 'path', 'command', 'query', 'pattern', 'glob', 'url'];
    const target = keys.map((key) => value[key]).find((item) => typeof item === 'string' && item.trim());
    return target ? target.trim().split(/\r?\n/, 1)[0].slice(0, 180) : '';
  }

  function activityLines(task) {
    const rows = activityRowsForTask(task).slice().sort((a, b) => Number(a.seq) - Number(b.seq));
    const calls = rows.filter((row) => row.kind === 'tool.call').map((row) => {
      const payload = row.payload && typeof row.payload === 'object' ? row.payload : {};
      const tool = typeof payload.tool === 'string' && payload.tool.trim() ? payload.tool.trim().slice(0, 80) : 'unknown';
      return { seq: Number(row.seq) || 0, tool, target: activityTarget(tool, payload.input), permission: '', failure: '', call: true };
    });
    const permissions = rows.filter((row) => row.kind === 'permission.decided');
    for (const row of permissions) {
      const payload = row.payload && typeof row.payload === 'object' ? row.payload : {};
      const tool = typeof payload.tool === 'string' && payload.tool.trim() ? payload.tool.trim().slice(0, 80) : 'unknown';
      const behavior = payload.behavior === 'allow' || payload.behavior === 'deny' ? payload.behavior : 'unknown';
      const seq = Number(row.seq) || 0;
      const candidates = calls.filter((call) => call.tool === tool && !call.permission);
      const match = candidates.find((call) => call.seq >= seq) || candidates.filter((call) => call.seq < seq).pop();
      if (match) match.permission = behavior;
      else calls.push({ seq, tool, target: '', permission: behavior, failure: '', call: false });
    }
    const failures = rows.filter((row) => row.kind === 'dispatch.failed');
    if (calls.length && failures.length) {
      const failed = failures[failures.length - 1]?.payload;
      const reason = failed?.reason || failed?.summary;
      if (typeof reason === 'string' && reason.trim()) {
        calls.sort((a, b) => a.seq - b.seq);
        calls[calls.length - 1].failure = reason.trim().split(/\r?\n/, 1)[0].slice(0, 180);
      }
    }
    return calls.sort((a, b) => a.seq - b.seq);
  }

  function activityDetailsHtml(task) {
    const lines = activityLines(task);
    if (!lines.length) return '';
    const hidden = Math.max(0, lines.length - 50);
    const shown = lines.slice(-50);
    const tools = lines.filter((line) => line.call).length;
    const denied = lines.filter((line) => line.permission === 'deny').length;
    const key = `${state.currentId}/${task.thread}/${task.i}`;
    const open = state.activityExpanded.has(key);
    return `
      <details class="tool-activity" data-activity-key="${esc(key)}"${open ? ' open' : ''}>
        <summary>What it ran · ${tools} ${tools === 1 ? 'tool' : 'tools'} · ${denied} denied</summary>
        <ul class="tool-rows">${shown.map((line) => `
          <li class="tool-row${line.failure ? ' failed' : ''}">
            <div class="tool-row-main"><span class="tool-name">${esc(line.tool || 'unknown')}</span>
              ${line.target ? `<code class="tool-target">${esc(line.target)}</code>` : ''}
              ${line.permission ? `<span class="tool-permission ${line.permission}">${esc(line.permission === 'deny' ? 'Denied' : line.permission === 'allow' ? 'Allowed' : 'Unknown')}</span>` : ''}
            </div>
            ${line.failure ? `<span class="tool-failure">Failed: ${esc(line.failure)}</span>` : ''}
          </li>`).join('')}</ul>
        ${hidden ? `<p class="tool-older">${fmt(hidden)} older ${hidden === 1 ? 'row' : 'rows'} hidden</p>` : ''}
      </details>`;
  }

  function replyHtml(task) {
    const message = shrinkText(task.message);
    const report = shrinkText(task.result);
    const files = task.kind === 'message' ? [] : artifactsFor(task);
    const status = task.status ? `<span class="chip ${statusClass(task.status)}">${esc(statusWord(task.status))}</span>` : '';
    return `
      <div class="reply">
        <div class="reply-k">
          <span class="dot ${statusClass(task.status) === 'bad' ? 'bad' : ''}"></span>
          <span class="who">${esc(task.thread)}</span>
          <span class="sep">·</span><span>${esc(task.kind === 'message' ? 'message' : 'round ' + (task.round ?? '—'))}</span>
          ${status}
        </div>
        ${message ? `<div class="say md">${md(message)}</div>` : '<p class="say turn-none">This turn has no message yet.</p>'}
        ${report ? `
          <div class="rep${statusClass(task.status) === 'bad' ? ' bad' : ''}">
            <div class="rep-k">${ic(statusClass(task.status) === 'bad' ? 'x' : 'check')}${esc(statusWord(task.status || 'reported'))}</div>
            <div class="rep-body md">${md(report)}</div>
          </div>` : ''}
        ${files.length ? `<div class="files">${files.map((a) => `
          <span class="fchip" title="${esc(a.path)}">${esc(String(a.path || '').split('/').pop())}${a.bytes != null ? ' · ' + bytes(a.bytes) : ''}</span>`).join('')}</div>` : ''}
        ${activityDetailsHtml(task)}
      </div>`;
  }

  // The agent is a choice, not a role: the run's goal names the agents it may use, and a thread can move.
  function agentPicker(thread) {
    const providers = state.detail?.goal?.providers || [];
    const open = state.agentMenu;
    const items = providers.map((p) => `
      <button class="apick-item${p.id === thread.agent ? ' is-on' : ''}" role="option"
              aria-selected="${p.id === thread.agent}" data-act="agent-set"
              data-agent="${esc(p.id)}" data-thread="${esc(thread.name)}">
        <span>${esc(p.id)}</span><span class="apick-drv">${esc(p.driver)}</span>
      </button>`).join('');
    return `
      <div class="apick">
        <button class="apick-btn" data-act="agent-menu" aria-haspopup="listbox" aria-expanded="${open}"
                aria-label="Agent for ${esc(thread.name)}: ${esc(thread.agent)}. Pick another for its next task.">
          ${esc(thread.agent)}${ic('chevron-down', 'ic sm')}
        </button>
        ${open ? `<div class="apick-menu" role="listbox" aria-label="Agents this run can use">${items ||
          '<p class="apick-none">This run names one agent.</p>'}</div>` : ''}
      </div>`;
  }

  async function setThreadAgent(thread, agent) {
    if (!state.currentId) return;
    try {
      await putJson(`/runs/${encodeURIComponent(state.currentId)}/threads/${encodeURIComponent(thread)}/agent`, { agent });
      state.agentMenu = false;
      await loadRun(state.currentId, { refresh: true });
      showToast(`${thread} now runs on ${agent}.`);
    } catch (err) {
      showToast(err.message);
      state.agentMenu = false;
    }
    renderAll();
  }

  // A stopped run has to say what stopped it and whether asking again would change anything. Resume is
  // offered only where it can; a spent token budget cannot be resumed by pressing a button.
  function runStopBanner() {
    const run = state.detail.run;
    const shapes = {
      blocked: { title: 'Run blocked', resume: true, fallback: 'The round did not pass, so nothing was merged.' },
      no_progress: { title: 'No progress', resume: true, fallback: 'Two rounds ended the same way, so nothing was merged.' },
      budget_exhausted: { title: 'Token budget spent', resume: false, fallback: 'The run reached its token budget, so nothing was merged.' },
      human_paused: { title: 'Run paused', resume: true, fallback: 'A person stopped this run. The turn in flight finished.' },
    };
    const shape = shapes[run.status];
    if (!shape) return '';
    const budget = state.detail.goal?.budget || {};
    const spent = `${fmt(run.tokens_used)}${budget.tokens ? ' of ' + fmt(budget.tokens) : ''} tokens`;
    return `
      <section class="runstop">
        <div class="runstop-head">${ic('alert-triangle')}<span>${esc(shape.title)}</span></div>
        <p>${esc(run.blocked_reason || shape.fallback)}</p>
        <div class="runstop-actions">
          ${shape.resume
            ? '<button class="btn" data-act="resume">Resume run</button>'
            : '<span class="runstop-next">A new run with a larger budget is the way forward.</span>'}
          <span class="runstop-meta">${spent} · ${roundsUsed()} round${roundsUsed() === 1 ? '' : 's'}</span>
        </div>
      </section>`;
  }

  function threadViewHtml() {
    const thread = currentThread();
    if (!thread) {
      return `
        <div class="empty-state">
          <span class="empty-gem" aria-hidden="true">◆</span>
          <p>This run has no threads. Its goal file names none, so there is nobody to talk to.</p>
          <p class="empty-note">Threads are created from the goal when the run starts.</p>
        </div>`;
    }
    const task = lastTask(thread);
    const open = openQuestion(thread.name);
    const status = open ? chip('needs you', 'warn') : task ? chip(statusWord(task.status || 'unknown'), statusClass(task.status)) : chip('no task', 'num');
    return `
      <div class="th-head">
        <div class="th-line">
          <h1 class="th-name">${esc(thread.name)}</h1>
          ${agentPicker(thread)}
          ${safetyChip(thread.agent)}
          ${status}
        </div>
        <p class="th-sub">${thread.tasks.length} ${thread.tasks.length === 1 ? 'task' : 'tasks'} in this run${task ? ` · last in round ${esc(task.round ?? '—')}` : ''}</p>
        ${thread.hasCheckout
          ? `<p class="th-where">${ic('branch', 'ic sm')}<span class="mono">${esc(thread.branch)}</span>${thread.commit ? `<span class="sep">·</span><span class="mono">${esc(thread.commit)}</span>` : ''}${thread.stat ? `<span class="sep">·</span><span>${esc(thread.stat.split('\n').pop().trim())}</span>` : ''}</p>`
          : '<p class="th-where muted">No worktree yet. It starts with this thread\'s first task.</p>'}
      </div>
      ${runStopBanner()}
      ${runIsWaiting()
        ? `<p class="waiting-note">${ic('alert-triangle', 'ic sm')}Nothing is running. The round is closed and no task is waiting — message a thread to start the next one.</p>`
        : ''}
      <div class="conv" role="log" aria-label="Conversation with ${esc(thread.name)}">
        ${thread.tasks.length
          ? thread.tasks.map((t) => `<article class="ex">${askHtml(t)}${replyHtml(t)}</article>`).join('')
          : `<p class="turn-none">No work yet. ${esc(thread.name)} has no task in this run. The box below starts its first one, as a message.</p>`}
      </div>`;
  }

  // ---- centre: board view ----------------------------------------------------------------------

  function ledgerHtml(e, { clamp = true, prefix = 'b', showRound = true } = {}) {
    const kind = ['intent', 'finding', 'question', 'decision', 'constraint', 'answer'].includes(e.kind) ? e.kind : '';
    const id = prefix + 'led-' + e.id;
    const expanded = state.expanded.has(String(id));
    return `
      <article class="led">
        <div><span class="kind ${kind}">${esc(e.kind || 'entry')}</span></div>
        <div>
          <div class="led-meta">
            <span class="who">by ${esc(e.author || e.thread || 'runner')}</span>
            ${showRound ? `<span class="sep"> · </span><span>round ${esc(e.round ?? '—')}</span>` : ''}
          </div>
          <p class="led-body${clamp && !expanded ? ' clamp' : ''}" id="${id}">${esc(shrinkText(e.body))}</p>
          ${clamp ? `<button class="led-more" data-act="more" data-target="${id}" aria-expanded="${expanded}" aria-controls="${id}">${expanded ? 'less' : 'more'}</button>` : ''}
          ${scopeChips(e.scope)}
        </div>
      </article>`;
  }

  function groupedLedger() {
    const { rounds = [], ledger = [] } = state.detail;
    if (!ledger.length) return [];
    const byRound = new Map();
    const loose = [];
    for (const entry of ledger) {
      if (entry.round === null || entry.round === undefined) {
        loose.push(entry);
        continue;
      }
      const key = Number(entry.round);
      byRound.set(key, [...(byRound.get(key) || []), entry]);
    }
    loose.sort((a, b) => Number(a.id) - Number(b.id));
    const indexes = Array.from(new Set([...rounds.map((r) => Number(r.index)), ...byRound.keys()])).sort((a, b) => a - b);
    const groups = indexes
      .filter((i) => (byRound.get(i) || []).length)
      .map((i) => ({
        label: `Round ${i}`,
        verdict: rounds.find((r) => Number(r.index) === i)?.verdict,
        entries: byRound.get(i).sort((a, b) => Number(a.id) - Number(b.id)),
      }));
    if (loose.length) groups.push({ label: 'Run', verdict: null, entries: loose });
    return groups;
  }

  function boardGroups() {
    const groups = groupedLedger();
    if (!groups.length) return '<p class="facts-empty">No board entries recorded.</p>';
    return groups.map((g) => `
      <section class="rgroup" aria-label="${esc(g.label)}">
        <div class="rgroup-head">
          <span class="rgroup-n">${esc(g.label)}</span>
          <span class="rgroup-c">${g.entries.length} ${g.entries.length === 1 ? 'entry' : 'entries'}</span>
          ${g.verdict ? chip(String(g.verdict).replace(/_/g, ' '), statusClass(g.verdict)) : ''}
        </div>
        ${g.entries.map((e) => ledgerHtml(e, { clamp: false, showRound: false })).join('')}
      </section>`).join('');
  }

  function boardViewHtml() {
    const d = state.detail;
    const ledger = d?.ledger || [];
    // The goal's own "done" text is the acceptance test, so it stays in view while the board is read.
    const done = d?.goal?.done
      ? `<p class="bview-done"><span class="k">Done when</span>${esc(shrinkText(d.goal.done))}</p>`
      : '';
    return `
      <div class="bview-head">
        <div class="bview-line">
          <h1>Board</h1>
          <span class="cnt">${ledger.length} ${ledger.length === 1 ? 'entry' : 'entries'} · ${esc(runTitle())}</span>
        </div>
        ${done}
      </div>
      ${runStopBanner()}
      ${boardGroups()}
      <details class="board-md">
        <summary class="md-summary">${ic('chevron-right', 'ic sm')}<span>board.md</span>${chip('what the agents read')}
          <span style="margin-left:auto">${state.board ? state.board.split('\n').length : 0} lines</span></summary>
        ${state.boardError ? `<p class="nr-error" role="alert">${esc(state.boardError)}</p>` : ''}
        <pre class="log-pre" tabindex="0" aria-label="board.md, read only">${esc(state.board || (state.boardError ? '' : 'The board is empty.'))}</pre>
      </details>`;
  }

  // ---- centre: diff view -----------------------------------------------------------------------

  // One row per thread that has a worktree. Each thread's patch is parsed once per commit.
  function diffThreads() {
    const runId = state.currentId;
    return (state.detail?.threads || []).filter((t) => t.has_checkout).map((t) => {
      const key = IlvunoDiff.cacheKey(runId, t);
      const entry = state.threadDiffs.get(key);
      if (entry && !entry.error && !state.parsedDiffs.has(key)) {
        state.parsedDiffs.set(key, IlvunoDiff.splitFiles(entry.patch));
      }
      return {
        name: t.name, branch: t.branch || '', commit: t.commit || '', stat: t.stat || '',
        entry, files: state.parsedDiffs.get(key) || [],
      };
    });
  }

  // The file to show: the one picked, else the current thread's first file, else the first file.
  function selectedDiff(threads) {
    const sel = state.diffSel;
    if (sel) {
      const thread = threads.find((t) => t.name === sel.thread);
      const file = thread?.files.find((f) => f.path === sel.path);
      if (file) return { thread, file };
    }
    const thread = threads.find((t) => t.name === state.thread && t.files.length) || threads.find((t) => t.files.length);
    return thread ? { thread, file: thread.files[0] } : null;
  }

  const KIND_LETTER = { added: 'A', deleted: 'D', renamed: 'R', modified: 'M' };
  const baseOf = (p) => String(p || '').split('/').pop();
  const dirOf = (p) => {
    const s = String(p || '');
    const i = s.lastIndexOf('/');
    return i >= 0 ? s.slice(0, i + 1) : '';
  };

  // Gate state, shown above the diff. The decision itself stays in the gate panel.
  function gateBarHtml() {
    const status = state.detail?.run?.status;
    if (status === 'reviewed') {
      return `<div class="dv-gate" role="status">
        <span class="chip warn">Gate open</span>
        <span>Accept or send back in the gate. Nothing merges until you accept.</span>
        <button class="btn sm" data-act="focus-gate">Open the gate</button>
      </div>`;
    }
    if (status === 'accepted' || status === 'merged') {
      return `<div class="dv-gate closed"><span class="chip ok">${esc(statusWord(status))}</span><span>The gate is closed for this run.</span></div>`;
    }
    return '';
  }

  function fileGroupHtml(thread, sel) {
    const isOn = (f) => !!sel && sel.thread.name === thread.name && sel.file.path === f.path;
    const head = `<div class="dv-thread"><span class="dv-thread-name">${esc(thread.name)}</span><span class="dv-thread-branch">${ic('branch', 'ic sm')}${esc(thread.branch || 'no branch')}</span></div>`;
    const err = thread.entry?.error ? `<p class="dv-note bad">Could not read this patch: ${esc(thread.entry.error)}</p>` : '';
    const rows = thread.files.map((f) => `
      <button class="dv-file${isOn(f) ? ' on' : ''}" data-act="diff-file" data-thread="${esc(thread.name)}" data-path="${esc(f.path)}"${isOn(f) ? ' aria-current="true"' : ''} title="${esc(f.path)}">
        <span class="dv-kind k-${esc(f.kind)}" title="${esc(f.kind)}">${KIND_LETTER[f.kind] || 'M'}</span>
        <span class="dv-path"><span class="dir">${esc(dirOf(f.path))}</span>${esc(baseOf(f.path))}</span>
        <span class="dv-counts"><span class="add">+${f.added}</span><span class="del">−${f.removed}</span></span>
      </button>`).join('');
    const empty = !thread.files.length && !err
      ? `<p class="dv-note">${thread.entry ? 'No file changes on this thread yet.' : 'Reading the patch…'}</p>`
      : '';
    return `<section class="dv-group" aria-label="Changes on ${esc(thread.name)}">${head}${err}${rows}${empty}</section>`;
  }

  function hunkHtml(hunk) {
    const rows = hunk.rows.map((r) => {
      const sign = r.type === 'add' ? '+' : r.type === 'del' ? '−' : '';
      return `<div class="dv-row ${r.type}"><span class="ln">${r.oldNo ?? ''}</span><span class="ln">${r.newNo ?? ''}</span><span class="sg">${sign}</span><code class="tx">${esc(r.text) || '&nbsp;'}</code></div>`;
    }).join('');
    return `<div class="dv-hunk"><div class="dv-hunk-head">${esc(hunk.header)}</div>${rows}</div>`;
  }

  function fileDiffHtml({ thread, file }) {
    const head = `<header class="dv-fhead">
        <span class="dv-fpath"><span class="dir">${esc(dirOf(file.path))}</span><b>${esc(baseOf(file.path))}</b></span>
        <span class="chip">${esc(file.kind)}</span>
        <span class="dv-counts"><span class="add">+${file.added}</span><span class="del">−${file.removed}</span></span>
        <span class="dv-on">${esc(thread.name)} · <code>${esc(thread.branch)}</code></span>
      </header>`;
    const cut = thread.entry?.truncated
      ? '<p class="dv-note warn">This patch was cut off at 128 KiB, so later files may be missing from the list.</p>'
      : '';
    let body;
    if (file.binary) body = '<p class="dv-note">Binary file. Its contents are not shown.</p>';
    else if (!file.hunks.length) body = '<p class="dv-note">No text changes in this file.</p>';
    else body = file.hunks.map(hunkHtml).join('');
    return `${head}${cut}<div class="dv-code" tabindex="0" aria-label="Changes in ${esc(file.path)}">${body}</div>`;
  }

  function diffViewHtml() {
    const threads = diffThreads();
    if (!threads.length) {
      return `
        <div class="empty-state">
          <span class="empty-gem" aria-hidden="true">◆</span>
          <p>No thread has a worktree in this run yet.</p>
          <p class="empty-note">A thread gets its branch and worktree when its first task starts.</p>
        </div>`;
    }
    const total = IlvunoDiff.summarize(threads.flatMap((t) => t.files));
    const sel = selectedDiff(threads);
    return `
      <div class="dv">
        ${gateBarHtml()}
        <div class="dv-head">
          <span class="dv-title">${total.files} ${total.files === 1 ? 'file' : 'files'} changed</span>
          <span class="dv-counts"><span class="add">+${fmt(total.added)}</span><span class="del">−${fmt(total.removed)}</span></span>
          <span class="dv-scope">across ${threads.length} ${threads.length === 1 ? 'thread' : 'threads'}</span>
        </div>
        <div class="dv-body">
          <nav class="dv-list" aria-label="Changed files">${threads.map((t) => fileGroupHtml(t, sel)).join('')}</nav>
          <section class="dv-view" aria-label="Diff of the selected file">${sel ? fileDiffHtml(sel) : '<p class="facts-empty">No file changes in these threads.</p>'}</section>
        </div>
      </div>`;
  }

  // ---- tab strip and status bar ----------------------------------------------------------------

  // The thread the tab strip and status bar talk about: the selected one, else the first.
  function focusThread() {
    const threads = runThreads();
    return threads.find((t) => t.name === state.thread) || threads[0] || null;
  }

  function renderTabs() {
    const bar = $('#tabbar');
    if (!bar) return;
    const d = state.detail;
    bar.hidden = !d;
    if (!d) {
      bar.innerHTML = '';
      return;
    }
    const diffs = diffThreads();
    const fileCount = diffs.reduce((n, t) => n + t.files.length, 0);
    const entries = (d.ledger || []).length;
    const tab = (name, label, count) => {
      const on = state.view === name;
      return `<button class="tab${on ? ' on' : ''}" role="tab" id="tab-${name}" aria-selected="${on}" aria-controls="view"
          tabindex="${on ? 0 : -1}" data-act="tab" data-tab="${name}">${label}${count ? `<span class="tab-n">${count}</span>` : ''}</button>`;
    };
    // In the Diff tab the branch shown is the open file's thread; elsewhere it is the selected thread.
    const shown = state.view === 'diff' && selectedDiff(diffs) ? selectedDiff(diffs).thread : focusThread();
    const where = shown?.branch
      ? `<span class="tab-where" title="Branch of ${esc(shown.name)}">${ic('branch', 'ic sm')}<span class="mono">${esc(shown.branch)}</span></span>`
      : '';
    bar.innerHTML = `<div class="tabs" role="tablist" aria-label="Run views">
        ${tab('thread', 'Conversation', '')}${tab('diff', 'Diff', fileCount ? fmt(fileCount) : '')}${tab('board', 'Board', entries ? fmt(entries) : '')}
      </div>
      <span class="tab-spacer"></span>${where}`;
  }

  function renderStatus() {
    const bar = $('#statusbar');
    if (!bar) return;
    if (!state.detail) {
      bar.innerHTML = `<span class="sb-item muted">${state.health === false ? 'Runner offline' : 'No run selected'}</span>
        <span class="sb-spacer"></span><span class="sb-item muted">Ctrl K for commands</span>`;
      return;
    }
    const thread = focusThread();
    const groups = groupedThreads();
    const files = diffThreads().reduce((n, t) => n + t.files.length, 0);
    const branch = thread?.hasCheckout
      ? `${ic('branch', 'ic sm')}<span class="mono">${esc(thread.branch)}</span>`
      : '<span class="muted">no branch yet</span>';
    bar.innerHTML = `
      <span class="sb-item">${branch}</span>
      <span class="sb-item"><span class="dot${groups.working.length ? ' ok' : ''}"></span>${groups.working.length} working</span>
      <span class="sb-item"><span class="dot${groups.needs.length ? ' warn' : ''}"></span>${groups.needs.length} ${groups.needs.length === 1 ? 'needs' : 'need'} you</span>
      <span class="sb-spacer"></span>
      <span class="sb-item muted">${fmt(files)} ${files === 1 ? 'file' : 'files'} changed</span>
      <span class="sb-item muted">Ctrl K for commands</span>`;
  }

  // ---- centre: no run --------------------------------------------------------------------------

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
          <p class="empty-note">A goal says what to do, what done looks like, and which agents may work on it.</p>
        </div>`;
    }
    return '<p class="facts-empty" style="padding:24px">Reading the run…</p>';
  }

  // ---- right rail ------------------------------------------------------------------------------

  function checksHtml() {
    const rounds = state.detail?.rounds || [];
    if (!rounds.length) return '<p class="facts-empty">No rounds recorded.</p>';
    const rows = rounds.map((r) => {
      const checks = r.readiness?.checks || [];
      // A round with a failed task never runs its checks, so `failing` is the only reason it has.
      const failing = (r.readiness?.failing || [])
        .filter((f) => String(f).startsWith('task-failed:'))
        .map((f) => String(f).replace(/^task-failed:/, 'task failed: '));
      if (!checks.length && !failing.length) return '';
      return `
        <div class="chk-round">Round ${esc(r.index)}${r.verdict ? ' · ' + esc(statusWord(r.verdict)) : ''}</div>
        ${checks.map((c) => `
          <div class="chk">
            <span class="name" title="${esc(c.detail || c.name)}">${esc(c.name)}</span>
            ${chip(c.passed ? 'pass' : 'fail', c.passed ? 'ok' : 'bad')}
          </div>`).join('')}
        ${failing.map((f) => `
          <div class="chk">
            <span class="name" title="${esc(f)}">${esc(f)}</span>${chip('fail', 'bad')}
          </div>`).join('')}`;
    }).join('');
    // The goal named no checks, so there is nothing to list and nothing to explain per round.
    return rows || '<p class="facts-empty">This goal named no checks.</p>';
  }

  function gatePanel() {
    const { rounds = [] } = state.detail;
    const round = rounds[rounds.length - 1] || { index: 0, readiness: { checks: [] } };
    const checks = round.readiness?.checks || [];
    const draft = state.noteDrafts.get(state.currentId) || '';
    const threads = state.detail.threads || [];
    const checkouts = threads.filter((thread) => thread.has_checkout);
    const fileCount = checkouts.reduce((sum, thread) => {
      const count = /(?:^|\s)(\d+) files? changed/.exec(thread.stat || '')?.[1];
      return sum + Number(count || 0);
    }, 0);
    const mergeLabel = checkouts.length
      ? `Merge ${checkouts.length} ${checkouts.length === 1 ? 'thread' : 'threads'} · ${fileCount} ${fileCount === 1 ? 'file' : 'files'}`
      : 'Accept · no files to merge';
    const threadDiffs = threads.map((thread) => {
      if (!thread.has_checkout) return '';
      const key = IlvunoDiff.cacheKey(state.currentId, thread);
      const diff = state.threadDiffs.get(key);
      const stat = thread.stat || 'No file changes';
      const branch = thread.branch || 'branch unknown';
      const patch = diff?.patch || '';
      const patchContent = diff?.error
        ? `Could not read this patch: ${diff.error}`
        : `${patch || 'No file changes.'}${diff?.truncated ? '\n\nPatch cut off at 128 KiB.' : ''}`;
      return `<article class="gate-thread">
        <div class="gate-thread-head"><b>${esc(thread.name)}</b><span>${esc(branch)}</span></div>
        <p class="gate-stat">${esc(stat)}</p>
        <details class="gate-diff" data-key="${esc(key)}"><summary>View patch</summary><pre>${esc(patchContent)}</pre></details>
      </article>`;
    }).join('');
    return `
      <section class="gate-panel" aria-label="Gate">
        <div class="gate-phead"><span class="gate-k">Gate · round ${esc(round.index)}</span>${chip('open', 'warn')}</div>
        <p class="gate-copy">${checks.length ? 'The checks passed.' : 'This round recorded no checks.'} ${checkouts.length ? 'Accept to merge these thread changes into the project.' : 'Accept to record the decision. This run has no Git checkouts.'}</p>
        ${threadDiffs ? `<div class="gate-threads">${threadDiffs}</div>` : ''}
        <div class="gate-checks">${checks.length ? checks.map((c) => `
          <div class="gate-check">
            <svg class="${c.passed ? 'ic' : 'ic bad'}" aria-hidden="true"><use href="#i-${c.passed ? 'check' : 'x'}"></use></svg>
            <span title="${esc(c.name)}">${esc(c.name)}</span>
            ${chip(c.passed ? 'pass' : 'fail', c.passed ? 'ok' : 'bad')}
          </div>`).join('') : '<div class="gate-check">No checks were recorded for this round. An empty list passes.</div>'}
        </div>
        <label class="field-label" for="gate-note">Note to the team <span>needed only to send back</span></label>
        <textarea class="gate-note" id="gate-note" rows="2" placeholder="What should change before this is accepted?">${esc(draft)}</textarea>
        <div class="gate-actions">
          <button class="btn primary" data-act="accept-gate"${state.gateBusy ? ' disabled' : ''}>${esc(mergeLabel)}</button>
          <button class="btn" data-act="send-back"${state.gateBusy ? ' disabled' : ''}>Send back with note</button>
        </div>
        <p class="gate-inline" id="gateInline" role="status" hidden></p>
      </section>`;
  }

  function gateReceipt() {
    const { run, rounds = [] } = state.detail;
    const passedRound = rounds.find((r) => r.verdict === 'pass');
    const checks = passedRound?.readiness?.checks || [];
    const n = checks.filter((c) => c.passed).length;
    const at = passedRound ? passedRound.index : Math.max(0, roundsUsed() - 1);
    const verb = run.status === 'merged' ? 'merged' : 'accepted';
    return `
      <div class="gate-bar">
        <span class="gate-ic">${ic('check')}</span>
        <div class="gate-t"><b>Gate closed · ${verb}</b><br>${verb === 'merged' ? 'Merged' : 'Accepted'} at round ${at}${checks.length ? ` after ${n} of ${checks.length} checks passed` : ''}.</div>
      </div>`;
  }

  function gateSection() {
    const run = state.detail.run;
    if (run.status === 'reviewed') return gatePanel();
    if (run.status === 'accepted' || run.status === 'merged') return gateReceipt();
    if (run.status === 'blocked' && String(run.blocked_reason || '').startsWith('merge conflict in:')) {
      const files = String(run.blocked_reason).replace(/^merge conflict in:\s*/, '');
      return `<div class="gate-conflict"><b>Merge stopped</b><p>The conflicting merge was aborted. The project was left as it was before that merge.</p><p>Files: ${esc(files)}</p></div>`;
    }
    if (['blocked', 'budget_exhausted', 'no_progress', 'human_paused'].includes(run.status)) {
      return `<p class="facts-empty">The gate did not open: the run is ${esc(statusWord(run.status))}. Its reason is in the centre panel.</p>`;
    }
    return `<p class="facts-empty">Nothing to decide. The gate opens when the round's checks pass.</p>`;
  }

  function gateCount() {
    const run = state.detail?.run;
    if (run?.status === 'reviewed') return 'open';
    return run?.status === 'accepted' || run?.status === 'merged' ? 'closed' : '—';
  }

  function factsHtml() {
    if (!state.detail) {
      return `
        <div class="fsec">
          <div class="fsec-head"><h2>Project</h2></div>
          <div class="path">${esc(shortProject() || '—')}</div>
          <div class="frow"><span>Runs</span><span class="v">${state.runs.length}</span></div>
        </div>
        <p class="facts-empty">${state.health === false
          ? 'The runner is not answering, so there is nothing to check.'
          : state.runs.length
            ? 'Pick a run to see its board, checks and gate.'
            : 'A run puts its board, checks and gate here.'}</p>`;
    }

    const { rounds = [], threads = [], ledger = [] } = state.detail;
    const checks = rounds.flatMap((r) => r.readiness?.checks || []);
    const passed = checks.filter((c) => c.passed).length;

    const threadRows = threads.map((t) => {
      const safety = safetyOf(t.agent);
      return `
        <div class="thread">
          <span class="dot ${t.status === 'idle' ? '' : 'ok'}"></span>
          <span class="th-name-sm">${esc(t.name)}</span>
          <span class="chip ${safety === 'unknown' ? '' : 'safe'}" title="${esc(safetyLine(safety))}">${esc(SAFETY_LABEL[safety] || 'Unknown')}</span>
          <span class="th-agent-sm">${esc(t.agent || 'unknown')}</span>
        </div>`;
    }).join('');

    // One agent for every thread is allowed — a single subscription has to work — but it is worth saying,
    // because a second agent is the reason this app exists.
    const agentsUsed = [...new Set(threads.map((t) => t.agent))];
    const sameAgentNote = threads.length > 1 && agentsUsed.length === 1
      ? `<p class="legend">Every thread runs on ${esc(agentsUsed[0])}. That works with one subscription; a second agent would add a second opinion.</p>`
      : '';

    const artifactList = (state.detail.artifacts || []).slice().sort((a, b) => roundOfArtifact(b) - roundOfArtifact(a));
    const artifactRows = artifactList.map((a) => {
      const parts = String(a.path || '').split('/');
      const fn = parts[parts.length - 1] || a.path;
      const m = String(a.path || '').match(/\/rounds\/r(\d+)\/([^/]+)\//);
      const loc = m ? `r${m[1]} · ${m[2]}` : (a.thread || 'artifact');
      return `
        <div class="art" title="${esc(a.path)}">
          <span class="fn">${esc(fn)}</span>
          <span class="sz">${a.bytes != null ? bytes(a.bytes) : '—'}</span>
          <span class="loc">${esc(loc)}</span>
        </div>`;
    }).join('');

    return `
      ${runFactsHtml(state.detail.run)}
      ${state.view === 'board' ? '' : `<section class="fsec" aria-labelledby="sec-board">
        <div class="fsec-head"><h2 id="sec-board">Board</h2><span class="n">${ledger.length}</span>
          <button class="fsec-link" data-act="board">View all</button></div>
        ${ledger.length ? ledger.slice().reverse().map((e) => ledgerHtml(e)).join('') : '<p class="facts-empty">No board entries yet.</p>'}
      </section>`}
      <section class="fsec" aria-labelledby="sec-checks">
        <div class="fsec-head"><h2 id="sec-checks">Checks</h2><span class="n">${checks.length ? `${passed} of ${checks.length}` : ''}</span></div>
        ${checksHtml()}
      </section>
      <section class="fsec" id="gate" tabindex="-1" aria-labelledby="sec-gate">
        <div class="fsec-head"><h2 id="sec-gate">Gate</h2><span class="n">${gateCount()}</span></div>
        ${gateSection()}
      </section>
      <section class="fsec" aria-labelledby="sec-artifacts">
        <div class="fsec-head"><h2 id="sec-artifacts">Artifacts</h2><span class="n">${artifactList.length}</span></div>
        ${artifactRows || '<p class="facts-empty">Nothing collected yet.</p>'}
      </section>
      <section class="fsec" aria-labelledby="sec-threads">
        <div class="fsec-head"><h2 id="sec-threads">Threads</h2><span class="n">${threads.length}</span></div>
        ${threadRows || '<p class="facts-empty">No threads yet.</p>'}
        ${sameAgentNote}
        <p class="legend">Ilvuno does not sandbox agents; each CLI is bounded by its own permissions.</p>
      </section>`;
  }

  function roundOfArtifact(a) {
    const m = String(a.path || '').match(/\/rounds\/r(\d+)\//);
    return m ? Number(m[1]) : -1;
  }

  function fmtDuration(ms) {
    const s = Math.max(0, Math.round(Number(ms) / 1000));
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m ${s % 60}s`;
    return `${Math.floor(m / 60)}h ${String(m % 60).padStart(2, '0')}m`;
  }

  // What the run is, in one block. Every value is a field on GET /runs/:id; nothing is estimated,
  // and a missing value shows a dash rather than a zero.
  function runFactsHtml(run) {
    const budget = state.detail?.goal?.budget || {};
    const started = Number(run.started_at || 0);
    const ranFor = started ? fmtDuration((run.ended_at ? Number(run.ended_at) : Date.now()) - started) : '—';
    const tokens = Number(run.tokens_used || 0);
    const cap = Number(budget.tokens || 0);
    const pct = cap ? Math.min(100, Math.round((tokens / cap) * 100)) : 0;
    return `
      <section class="fsec" aria-label="Run facts">
        <div class="frow"><span>Folder</span><span class="v" title="${esc(run.project || '')}">${esc(tailPath(run.project) || '—')}</span></div>
        <div class="frow"><span>Id</span><span class="v" title="${esc(run.id || '')}">${esc(String(run.id || '—').slice(0, 8))}</span></div>
        <div class="frow"><span>Ran for</span><span class="v">${esc(ranFor)}</span></div>
        <div class="frow"><span>Rounds</span><span class="v"><b>${roundsUsed()}</b>${budget.rounds ? ' of ' + esc(budget.rounds) : ''}</span></div>
        <div class="frow"><span>Tokens</span><span class="v"><b>${compact(tokens)}</b>${cap ? ' of ' + compact(cap) : ''}</span></div>
        ${cap ? `<div class="bar${pct >= 90 ? ' hot' : ''}" role="img" aria-label="${pct}% of this run's token budget used"><i style="width:${pct}%"></i></div>` : ''}
      </section>`;
  }

  function renderFacts() {
    const body = $('#factsBody');
    if (!body) return;
    const top = body.scrollTop;
    // A refresh rebuilds the panel. Keep any open patch open, so reading one is not interrupted by polling.
    const openPatches = new Set($$('details.gate-diff[open]', body).map((d) => d.dataset.key));
    body.innerHTML = factsHtml();
    $$('details.gate-diff', body).forEach((d) => { if (openPatches.has(d.dataset.key)) d.open = true; });
    if (state.lastRunId === state.currentId) body.scrollTop = top;
    else body.scrollTop = 0;
    $('#factsLbl').textContent = state.detail ? 'Run' : 'No run';
  }

  // ---- centre render ---------------------------------------------------------------------------

  function renderView() {
    const view = $('#view');
    const scroll = $('#scroll');
    if (!view || !scroll) return;
    scroll.classList.toggle('is-fill', !!state.detail && state.view === 'diff');
    const top = scroll.scrollTop;
    $$('details[data-activity-key]', view).forEach((details) => {
      const key = details.dataset.activityKey;
      if (details.open) state.activityExpanded.add(key);
      else state.activityExpanded.delete(key);
    });
    const key = `${state.currentId}|${state.view}|${state.thread}`;
    const same = key === state.lastThreadKey;

    if (!state.detail) {
      view.className = 'view';
      view.innerHTML = emptyState();
      scroll.scrollTop = 0;
    } else if (state.view === 'diff') {
      // The file list and the code scroll on their own; keep both where they were across a refresh.
      const panes = { list: $('.dv-list', view)?.scrollTop, code: $('.dv-view', view)?.scrollTop };
      view.className = 'view diff-view';
      view.innerHTML = diffViewHtml();
      scroll.scrollTop = 0;
      if (panes.list != null && $('.dv-list', view)) $('.dv-list', view).scrollTop = panes.list;
      if (panes.code != null && $('.dv-view', view)) $('.dv-view', view).scrollTop = panes.code;
    } else if (state.view === 'board') {
      view.className = 'view wide';
      view.innerHTML = boardViewHtml();
      if (same) scroll.scrollTop = top;
      else scroll.scrollTop = 0;
    } else {
      view.className = 'view';
      view.innerHTML = threadViewHtml();
      if (same) scroll.scrollTop = top;
      else pinBottom(scroll);
    }
    state.lastThreadKey = key;
    requestAnimationFrame(initExpanders);
  }

  // A thread opens at its newest turn. Pin again next frame and once the fonts are in: a font swap
  // changes every height, and without the second pin the pane rests half-way up an old message.
  function pinBottom(el) {
    const stick = () => { el.scrollTop = el.scrollHeight; };
    stick();
    requestAnimationFrame(() => requestAnimationFrame(stick));
    document.fonts?.ready.then(stick);
  }

  function initExpanders() {
    $$('.led-body.clamp').forEach((p) => {
      const btn = p.parentElement.querySelector(`[data-act="more"][data-target="${p.id}"]`);
      if (!btn) return;
      const over = p.scrollHeight > p.clientHeight + 1;
      btn.hidden = !over;
      if (!over) p.classList.remove('clamp');
    });
  }

  // ---- composer --------------------------------------------------------------------------------

  function composerReason(thread) {
    const run = state.detail?.run;
    if (state.health === false) return 'The runner is not answering.';
    if (!thread) return 'No thread is selected.';
    if (!run) return 'No run is selected.';
    if (run.status !== 'working') {
      // A stopped run is not finished: Resume picks it up, and saying "start a new run" would hide that.
      const resumable = ['blocked', 'budget_exhausted', 'no_progress', 'human_paused'].includes(run.status);
      return `This run is ${statusWord(run.status)}. ${resumable ? 'Resume it to continue.' : 'Start a new run to continue.'}`;
    }
    return '';
  }

  function autosizeText(el) {
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 150) + 'px';
  }

  // What a person can do when the run cannot take a message. A finished run takes a new one; a paused
  // run is picked up again. The bars that offer these live in runStopBanner, so this only adds the move.
  function composerNextAction() {
    const run = state.detail?.run;
    if (!run || state.health === false) return '';
    if (['blocked', 'budget_exhausted', 'no_progress', 'human_paused'].includes(run.status)) {
      return '<button class="btn" data-act="resume">Resume</button>';
    }
    return '<button class="btn" data-act="new-run">New run</button>';
  }

  function renderComposer() {
    const form = $('#composer');
    const thread = currentThread();
    const show = !!state.detail && state.view === 'thread' && !!thread;
    form.hidden = !show;
    if (!show) return;

    const text = $('#composerText');
    const send = $('[data-act="send-message"]');
    const box = $('#composerBox');
    const note = $('#composerNote');
    const error = $('#composerError');
    const closed = $('#composerClosed');
    const draft = state.drafts.get(thread.name) || '';
    if (text.value !== draft) text.value = draft;
    autosizeText(text);

    const reason = composerReason(thread);
    const usable = !reason && !state.sending;
    // The run cannot take a message, so the box goes and the reason takes its place. A box you cannot
    // type in says "you are missing something"; one sentence says what happened and what to do.
    const shut = !!reason;
    text.disabled = !usable;
    text.placeholder = `Message ${thread.name}…`;
    send.disabled = !usable || !draft.trim();
    box.hidden = shut;
    note.hidden = shut;
    form.classList.toggle('closed', shut);
    closed.hidden = !shut;
    closed.innerHTML = shut ? `<span>${esc(reason)}</span>${composerNextAction()}` : '';

    note.textContent = thread.tasks.length
      ? `Ctrl+Enter sends. It arrives as a message task on ${thread.name}.`
      : `${thread.name} has no task yet. Sending starts its first one, as a message.`;

    const err = state.sendError && state.sendError.thread === thread.name ? state.sendError.message : '';
    error.hidden = !err;
    error.textContent = err;
  }

  /// Stop a run that is working. The runner lets the turn in flight finish, so this reports "stopping"
  /// until the run's own status changes; the banner then says who stopped it and offers Resume.
  async function stopRun() {
    if (!state.currentId || state.stopping) return;
    state.stopping = true;
    renderChrome();
    try {
      await postJson(`/runs/${encodeURIComponent(state.currentId)}/stop`, {});
      showToast('Stopping after the current turn.');
      await loadRun(state.currentId, { refresh: true });
      await refreshRuns();
    } catch (err) {
      showToast(err.message);
    }
    state.stopping = false;
    renderAll();
  }

  async function sendMessage() {
    const thread = currentThread();
    const text = $('#composerText');
    if (!thread || !text || state.sending) return;
    const prompt = (state.drafts.get(thread.name) || text.value || '').trim();
    if (!prompt) return;
    const reason = composerReason(thread);
    if (reason) {
      state.sendError = { thread: thread.name, message: reason };
      renderComposer();
      return;
    }
    state.sending = true;
    state.sendError = null;
    renderComposer();
    try {
      await postJson(`/runs/${encodeURIComponent(state.currentId)}/tasks`, { thread: thread.name, prompt, kind: 'message' });
      state.drafts.set(thread.name, '');
      state.sending = false;
      await loadRun(state.currentId, { refresh: true });
      await refreshRuns();
      showToast(`Message sent to ${thread.name}.`);
      $('#composerText')?.focus({ preventScroll: true });
    } catch (err) {
      state.sending = false;
      state.sendError = { thread: thread.name, message: err.message };
    }
    renderAll();
  }

  // ---- panel layout ----------------------------------------------------------------------------

  // Panel sizes and open flags are a per-viewer convenience, kept in localStorage. Every access is
  // guarded: a window without storage still lays out, it just forgets its sizes.
  const LAYOUT_KEY = 'ilvuno-layout';
  const RAIL_LIMITS = [200, 380];
  const FACTS_LIMITS = [260, 480];
  const MAIN_MIN = 520; // a panel that would squeeze the centre below this is hidden, not shrunk
  const layout = { rail: 272, facts: 330, railOpen: true, factsOpen: window.innerWidth > 1280 };
  const clamp = (v, [lo, hi]) => Math.min(hi, Math.max(lo, v));

  function loadLayout() {
    try {
      const saved = JSON.parse(localStorage.getItem(LAYOUT_KEY) || '{}');
      if (Number.isFinite(saved.rail)) layout.rail = clamp(saved.rail, RAIL_LIMITS);
      if (Number.isFinite(saved.facts)) layout.facts = clamp(saved.facts, FACTS_LIMITS);
      if (typeof saved.railOpen === 'boolean') layout.railOpen = saved.railOpen;
      if (typeof saved.factsOpen === 'boolean') layout.factsOpen = saved.factsOpen;
    } catch (err) {
      // Unreadable storage: the defaults stand.
    }
  }

  function saveLayout() {
    try {
      localStorage.setItem(LAYOUT_KEY, JSON.stringify({
        rail: layout.rail, facts: layout.facts, railOpen: layout.railOpen, factsOpen: layout.factsOpen,
      }));
    } catch (err) {
      // No storage, no memory of the sizes; the layout still works.
    }
  }

  // The one place the grid is decided. CSS reads the two column widths; the handles follow them.
  function applyLayout() {
    const app = $('#app');
    if (!app) return;
    // The threads panel keeps its saved width until the window is too narrow for it, then gives way.
    const railW = layout.railOpen ? Math.min(layout.rail, Math.max(RAIL_LIMITS[0], window.innerWidth - MAIN_MIN)) : 0;
    const factsW = state.rightOpen && window.innerWidth - railW - layout.facts >= MAIN_MIN ? layout.facts : 0;
    app.style.setProperty('--rail-col', `${railW}px`);
    app.style.setProperty('--facts-col', `${factsW}px`);
    app.dataset.rail = layout.railOpen ? 'open' : 'closed';
    app.dataset.right = factsW ? 'open' : '';
    // The panels stay in the grid when closed: a 0px column hides them, and display:none would shift the
    // centre into the wrong column. Visibility and inert keep them out of sight and out of the tab order.
    setInert($('#rail'), !layout.railOpen);
    setInert($('.facts'), !factsW);
    $('#resizeRail').hidden = !layout.railOpen;
    $('#resizeFacts').hidden = !factsW;
    $('#resizeRail').setAttribute('aria-valuenow', String(layout.rail));
    $('#resizeFacts').setAttribute('aria-valuenow', String(layout.facts));
    $('.rail-toggle').setAttribute('aria-expanded', String(layout.railOpen));
    $('.facts-toggle').setAttribute('aria-expanded', String(!!factsW));
  }

  // Drag a handle to resize. Pointer capture keeps the drag alive when the pointer leaves the handle.
  function startResize(side, event) {
    const handle = event.currentTarget;
    event.preventDefault();
    handle.setPointerCapture?.(event.pointerId);
    handle.classList.add('dragging');
    const move = (ev) => {
      if (side === 'rail') layout.rail = clamp(ev.clientX, RAIL_LIMITS);
      else layout.facts = clamp(window.innerWidth - ev.clientX, FACTS_LIMITS);
      applyLayout();
    };
    const stop = () => {
      handle.classList.remove('dragging');
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
      saveLayout();
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  }

  // Arrow keys on a focused handle: 16px a step, 48px with Shift.
  function nudgeResize(handle, key, shift) {
    const step = (shift ? 48 : 16) * (key === 'ArrowRight' ? 1 : -1);
    if (handle.id === 'resizeRail') layout.rail = clamp(layout.rail + step, RAIL_LIMITS);
    else layout.facts = clamp(layout.facts - step, FACTS_LIMITS);
    applyLayout();
    saveLayout();
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
    $('#runMeta').innerHTML = runMetaHtml();
    $('#settingsVersion').textContent = state.version ? `v${state.version}` : 'Ilvuno';
    document.title = state.detail ? `${runTitle()} — Ilvuno` : 'Ilvuno';

    applyLayout();
    const app = $('#app');

    const offline = state.health === false;
    $('#offlineBanner').hidden = !offline;
    app.classList.toggle('is-offline', offline);
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
    $('#drawerState').innerHTML = `${t.kind === 'message' ? chip('message') : ''}${t.status ? chip(statusWord(t.status), statusClass(t.status)) : ''}`;

    const arts = artifactsFor(t);
    const scope = (t.scope || []).length
      ? t.scope.map((p) => p === 'run'
        ? `<div class="dfile">${ic('folder')}<span>Whole run</span></div>`
        : `<div class="dfile" title="${esc(p)}">${ic('folder')}<span>${esc(relPath(p))}</span></div>`).join('')
      : '<div class="dnone">No files declared. A message task has no scope.</div>';

    const produced = arts.length
      ? arts.map((a) => {
        const fn = String(a.path || '').split('/').pop();
        return `<div class="dfile" title="${esc(a.path)}">${ic('folder')}<span>${esc(fn)}</span><span class="sz">${a.bytes != null ? bytes(a.bytes) : '—'}</span></div>`;
      }).join('')
      : '<div class="dnone">No artifact was collected for this task.</div>';

    $('#drawerBody').innerHTML = `
      <div class="dsec">
        <h3>Prompt <span>round ${esc(t.round ?? '—')}</span></h3>
        <div class="dprompt">${esc(shrinkText(t.prompt))}</div>
      </div>
      <div class="dsec">
        <h3>What the agent said <span>${fmt((t.message || '').length)} chars</span></h3>
        <div class="dmsg md" tabindex="0">${md(shrinkText(t.message || 'No message.'))}</div>
      </div>
      <div class="dsec">
        <h3>Report <span>by ${esc(t.thread)}</span></h3>
        <div class="dreport">
          <div class="ok">${ic('check')} ${esc(statusWord(t.status || 'reported'))}</div>
          <div class="md">${md(shrinkText(t.result || 'No result line.'))}</div>
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
    const opt = (value, label) => `<label><input type="radio" name="policy" value="${value}" data-setting="message_policy"${current === value ? ' checked' : ''} disabled><span>${label}</span></label>`;
    return `<div class="seg is-disabled" role="radiogroup" aria-label="Message policy (not enforced yet)">${opt('never', 'Never')}${opt('round', 'At round end')}${opt('anytime', 'Anytime')}</div>`;
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
                <span class="agent-ver">${a.found ? esc(agentVersion(a)) : '—'}</span>
                <span class="agent-state"><span class="dot ${a.found ? 'ok' : 'bad'}"></span>${a.found ? 'found' : 'missing'}</span>
              </div>
              <div class="agent-path" title="${esc(a.path || '')}">${esc(a.path || 'not on the PATH of the ilvunod process')}</div>
              <div class="agent-safety">${esc(safetyLine(a.safety))}</div>
            </div>`).join('') || '<p class="facts-empty">The API reported no agents.</p>'}
        </div>
        <div class="note">Ilvuno does not sandbox agents; each CLI is bounded by its own permissions.</div>`;
    }
    if (tab === 'defaults') {
      const s = state.settings;
      if (!s) return state.settingsError ? `<p class="nr-error" role="alert">${esc(state.settingsError)}</p>` : '<p class="settings-loading">Reading settings…</p>';
      return `
        ${state.settingsError ? `<p class="nr-error" role="alert">${esc(state.settingsError)}</p>` : ''}
        <p class="set-sub">Not used yet. The runner reads the budget and message policy from each run's goal file, so these values change nothing today.</p>
        <div class="set-sec">
          <div class="k">Budget <span class="chip">Not enforced yet</span></div>
          <div class="form-row">
            <div class="form-label">Rounds<small>Set in each run's goal file.</small></div>
            <input class="num-input" type="number" min="1" max="20" value="${esc(s.budget_rounds)}" aria-label="Default rounds" disabled>
          </div>
          <div class="form-row">
            <div class="form-label">Tokens<small>Set in each run's goal file.</small></div>
            <input class="num-input" type="text" inputmode="numeric" value="${esc(fmt(s.budget_tokens))}" aria-label="Default tokens" disabled>
          </div>
        </div>
        <div class="set-sec">
          <div class="k">Messages <span class="chip">Not enforced yet</span></div>
          <div class="form-row">
            <div class="form-label">Message policy<small>Set per run in its goal file. The agents are told the policy in their prompt; nothing checks it.</small></div>
            ${policySeg(s.message_policy)}
          </div>
        </div>
        <div class="set-sec">
          <div class="k">Gate</div>
          <div class="form-row">
            <div class="form-label">Human gate<small>Every run stops at the gate. Nothing merges until you accept it.</small></div>
            <span class="chip ok">Always on</span>
          </div>
        </div>
        <div class="note">Theme is the only setting that takes effect today. Budgets and policy are saved but no run reads them yet.</div>`;
    }
    if (tab === 'about') {
      return `
        <p class="set-sub">A local runner for coding agents. One project, a small team of threads, and a record of every handoff.</p>
        <div class="set-sec">
          <div class="kv"><span>Version</span><span class="v">${esc(state.appVersion || '—')}</span></div>
          <div class="kv"><span>Runs on this machine</span><span class="v">${state.runs.length}</span></div>
          <div class="kv"><span>Local API</span><span class="v">127.0.0.1:4737</span></div>
          <div class="kv"><span>Data folder</span><span class="v">ILVUNO_HOME or ~/.local/share/ilvuno</span></div>
        </div>
        <div class="note">The window starts its bundled runner when the local API is not running. The runner stays active after the window closes.</div>
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
        <div class="k">Run</div>
        <div class="kv"><span>Run id</span><span class="v" title="${esc(state.currentId || '')}">${esc(String(state.currentId || '—').slice(0, 8))}</span></div>
        <div class="kv"><span>Threads</span><span class="v">${state.detail?.threads?.length || 0}</span></div>
      </div>
      <div class="set-sec">
        <div class="k">Keyboard</div>
        <div class="kv"><span>Command palette</span><span class="keys"><span class="kbd">Ctrl K</span></span></div>
        <div class="kv"><span>New run</span><span class="keys"><span class="kbd">Ctrl N</span></span></div>
        <div class="kv"><span>Send a message</span><span class="keys"><span class="kbd">Ctrl ↵</span></span></div>
        <div class="kv"><span>Conversation, Diff, Board</span><span class="keys"><span class="kbd">Ctrl 1</span><span class="kbd">Ctrl 2</span><span class="kbd">Ctrl 3</span></span></div>
        <div class="kv"><span>Threads panel</span><span class="keys"><span class="kbd">Ctrl B</span></span></div>
        <div class="kv"><span>Run work panel</span><span class="keys"><span class="kbd">Ctrl .</span></span></div>
        <div class="kv"><span>Next or previous thread or file</span><span class="keys"><span class="kbd">↑</span><span class="kbd">↓</span></span></div>
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
      renderRail();
      renderFacts();
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
    const canResume = ['blocked', 'budget_exhausted', 'no_progress', 'human_paused'].includes(status);
    const out = [{ act: 'new-run', label: 'New run', icon: 'plus', keys: 'new run start', on: true }];
    if (state.view !== 'board') out.push({ act: 'board', label: 'Show the board', icon: 'board', keys: 'board entries ledger view all', on: !!state.detail });
    if (state.view === 'board') out.push({ act: 'back-thread', label: 'Back to the conversation', icon: 'message', keys: 'conversation thread back', on: !!currentThread() || !!state.thread });
    if (state.detail && state.view !== 'diff') out.push({ act: 'show-diff', label: 'Show the diff', icon: 'diff', keys: 'diff changes files patch review', on: true });
    if (state.detail && state.view !== 'thread') out.push({ act: 'show-thread', label: 'Show the conversation', icon: 'message', keys: 'conversation thread chat', on: true });
    out.push({ act: 'toggle-rail', label: layout.railOpen ? 'Hide the threads panel' : 'Show the threads panel', icon: 'sidebar', keys: 'threads panel sidebar rail', on: true });
    out.push({ act: 'toggle-right', label: state.rightOpen ? 'Hide the run work panel' : 'Show the run work panel', icon: 'panel-right', keys: 'run work panel inspector facts', on: true });
    if (atGate) {
      out.push({ act: 'accept-gate', label: 'Accept and merge', icon: 'check', keys: 'accept merge gate done', on: true });
      out.push({ act: 'send-back', label: 'Send back with a note', icon: 'x', keys: 'send back reject gate note', on: true });
    }
    if (canResume) out.push({ act: 'resume', label: 'Resume run', icon: 'chevron-right', keys: 'resume continue blocked budget', on: true });
    out.push({ act: 'settings', label: 'Settings', icon: 'settings', keys: 'settings preferences agents', on: true });
    out.push({ act: 'toggle-theme', label: `Toggle theme (${resolveTheme(state.settings?.theme || 'dark') === 'dark' ? 'to light' : 'to dark'})`, icon: 'moon', keys: 'theme dark light toggle', on: true });
    return out;
  }

  let palActive = 0;

  function paletteHtml() {
    const runs = state.runs.length
      ? `<div class="pal-section" data-group="Runs"><div class="pal-group">Runs</div>
          ${state.runs.map((r) => `
            <button class="pal-item" id="pal-run-${esc(r.id)}" data-act="palette-run" data-id="${esc(r.id)}" data-search="${esc(`${r.title || ''} ${statusWord(r.status)} ${projectName(r.project)}`.toLowerCase())}" role="option" aria-selected="false">
              ${ic('folder')}
              <span class="lbl">${esc(r.title || 'Untitled run')}</span>
              <span class="hint">${esc(statusWord(r.status))}</span>
            </button>`).join('')}
        </div>`
      : '';
    const threads = runThreads();
    const threadSection = threads.length
      ? `<div class="pal-section" data-group="Threads"><div class="pal-group">Threads · this run</div>
          ${threads.map((t, i) => `
            <button class="pal-item" id="pal-thread-${i}" data-act="thread" data-name="${esc(t.name)}" data-search="${esc(`${t.name} ${t.agent} ${(lastTask(t)?.prompt || '')}`.toLowerCase())}" role="option" aria-selected="false">
              ${ic('message')}
              <span class="lbl">${esc(t.name)}</span>
              <span class="hint">${esc(t.agent)}</span>
            </button>`).join('')}
        </div>`
      : '';
    const actions = paletteActions().map((a, i) => `
      <button class="pal-item" id="pal-act-${i}" data-act="${a.act}" data-search="${esc(a.label.toLowerCase() + ' ' + a.keys)}" role="option" aria-selected="false"${a.on ? '' : ' aria-disabled="true"'}>
        ${ic(a.icon)}
        <span class="lbl">${esc(a.label)}</span>
      </button>`).join('');
    return `${runs}${threadSection}<div class="pal-section" data-group="Actions"><div class="pal-group">Actions</div>${actions}</div><div class="pal-empty" id="palEmpty" hidden>No matches.</div>`;
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
    return $$('.pal-item', $('#palList')).filter((el) => !el.hidden && !el.closest('[hidden]'));
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
    if (countEl) countEl.textContent = q ? `${count} result${count === 1 ? '' : 's'}` : `${state.runs.length} run${state.runs.length === 1 ? '' : 's'} · ${runThreads().length} thread${runThreads().length === 1 ? '' : 's'} · ${paletteActions().length} actions`;
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

  // The goal lives in a file. The picker returns a path; the window asks the shell to read it, because
  // the page itself has no file access.
  async function openGoalFrom(path) {
    if (!path) return;
    try {
      const text = await window.__TAURI__.core.invoke('read_goal', { path });
      JSON.parse(text); // a goal must be JSON; the runner says the rest when Start is pressed
      const area = $('#nr-goal');
      if (area) area.value = text;
      const shown = $('#nr-path');
      if (shown) shown.value = path;
      newRunError('');
      showToast(`Read ${path.split('/').pop()}.`);
    } catch (err) {
      newRunError(err && err.message ? err.message : String(err));
    }
  }

  async function browseGoal() {
    if (!window.__TAURI__ || !window.__TAURI__.dialog) {
      newRunError('This window has no file picker. Type the path and press Open.');
      return;
    }
    try {
      const picked = await window.__TAURI__.dialog.open({
        multiple: false,
        directory: false,
        filters: [{ name: 'Goal', extensions: ['json'] }],
      });
      if (picked) await openGoalFrom(typeof picked === 'string' ? picked : picked.path);
    } catch (err) {
      newRunError(err && err.message ? err.message : String(err));
    }
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
      state.view = 'thread';
      state.thread = null;
      state.drawerIndex = -1;
      state.lastRunId = null;
      state.expanded = new Set();
      state.lastThreadKey = '';
      setRoute('thread');
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
    renderFacts();
    renderChrome();
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
      renderFacts();
      renderChrome();
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

  function selectThread(name) {
    if (!name) return;
    state.thread = name;
    state.view = 'thread';
    state.sendError = null;
    setRoute('t/' + name);
  }

  // Conversation keeps the selected thread; Diff and Board are whole-run views. The hash carries the view.
  function switchTab(name) {
    if (!state.detail) return;
    if (name === 'thread') setRoute(state.thread ? 't/' + state.thread : 'thread');
    else setRoute(name);
  }

  // Arrow keys move through a thread list or a file list, and the item moved to is opened at once.
  function moveInList(list, from, step) {
    const selector = list.classList.contains('dv-list') ? '.dv-file' : '.trow';
    const items = $$(selector, list);
    const i = items.indexOf(from.closest(selector));
    if (i < 0 || !items.length) return;
    const next = items[Math.min(items.length - 1, Math.max(0, i + step))];
    next.focus({ preventScroll: true });
    next.click();
  }

  function focusGate() {
    if (!state.rightOpen) {
      state.rightOpen = true;
      renderChrome();
    }
    const el = $('#gate');
    if (!el) return;
    el.scrollIntoView({ block: 'center' });
    el.classList.add('flash');
    setTimeout(() => el.classList.remove('flash'), 1400);
    el.focus({ preventScroll: true });
  }

  // ---- render all ------------------------------------------------------------------------------

  function focusToken() {
    const node = document.activeElement;
    if (!node || !node.closest || !node.closest('#app')) return null;
    const d = node.dataset;
    if (!d || !d.act) return null;
    return { act: d.act, name: d.name, i: d.i, id: d.id, tab: d.tab, target: d.target, path: d.path, thread: d.thread };
  }

  function restoreFocus(token) {
    if (!token) return;
    const all = $$(`[data-act="${token.act}"]`);
    const el = all.find((n) => ['name', 'i', 'id', 'tab', 'target', 'path', 'thread']
      .every((key) => token[key] === undefined || n.dataset[key] === token[key]));
    if (!el || el.closest('[inert]')) return;
    setTimeout(() => el.focus({ preventScroll: true }), 0);
  }

  function viewSignature() {
    return JSON.stringify({
      id: state.currentId, detail: state.detail, board: state.board, boardError: state.boardError,
      runs: state.runs, runsError: state.runsError, health: state.health, version: state.version,
      conn: state.conn, view: state.view, thread: state.thread, right: state.rightOpen,
      drawer: state.drawerIndex, settings: state.settings, settingsError: state.settingsError,
      settingsTab: state.settingsTab, agents: state.agents, agentsError: state.agentsError,
      agentsLoading: state.agentsLoading, gateBusy: state.gateBusy, sending: state.sending,
      sendError: state.sendError, expanded: Array.from(state.expanded),
      note: Array.from(state.noteDrafts.entries()), route: routeName(),
      diffSel: state.diffSel, diffs: Array.from(state.threadDiffs.keys()),
    });
  }

  let lastSignature = '';

  function renderAll(force = false) {
    const signature = viewSignature();
    if (!force && signature === lastSignature) return;
    const focused = focusToken();

    renderRail();
    renderTabs();
    renderView();
    renderComposer();
    renderFacts();
    renderChrome();
    renderStatus();

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
    lastSignature = signature;
  }

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
      state.thread = null;
      state.expanded = new Set();
      state.sendError = null;
    }
    if (state.view !== 'board') state.view = 'thread';
    connectEvents(id);
    setRoute(homeRoute());
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
      case 'thread':
        if (dialogName() === 'palette') {
          selectThread(target.dataset.name);
          renderAll();
        } else {
          selectThread(target.dataset.name);
        }
        break;
      case 'palette-run':
        state.view = 'thread';
        selectRun(target.dataset.id);
        break;
      case 'tab':
        if (dialogName() === 'palette') closeDialog();
        switchTab(target.dataset.tab);
        break;
      case 'diff-file':
        state.diffSel = { thread: target.dataset.thread, path: target.dataset.path };
        renderAll();
        break;
      case 'board':
        if (dialogName() === 'palette') state.lastDialog = 'palette';
        state.view = 'board';
        setRoute('board');
        break;
      case 'back-thread':
        if (dialogName() === 'palette') state.lastDialog = 'palette';
        state.view = 'thread';
        setRoute(homeRoute());
        break;
      case 'task':
        state.opener = target;
        state.drawerIndex = Number(target.dataset.i) || 0;
        setRoute('task-' + (state.drawerIndex + 1));
        break;
      case 'settings-tab':
        setRoute('settings-' + target.dataset.tab);
        break;
      case 'toggle-right': {
        if (dialogName() === 'palette') closeDialog();
        state.rightOpen = !state.rightOpen;
        layout.factsOpen = state.rightOpen;
        saveLayout();
        renderChrome();
        break;
      }
      case 'toggle-rail':
        if (dialogName() === 'palette') closeDialog();
        layout.railOpen = !layout.railOpen;
        saveLayout();
        applyLayout();
        break;
      case 'show-diff':
      case 'show-thread':
        if (dialogName() === 'palette') closeDialog();
        switchTab(act === 'show-diff' ? 'diff' : 'thread');
        break;
      case 'stop-run':
        stopRun();
        break;
      case 'agent-menu':
        state.agentMenu = !state.agentMenu;
        renderView();
        break;
      case 'agent-set':
        setThreadAgent(target.dataset.thread, target.dataset.agent);
        break;
      case 'focus-gate':
        focusGate();
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
      case 'browse-goal':
        browseGoal();
        break;
      case 'open-goal':
        openGoalFrom($('#nr-path')?.value.trim());
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

  document.addEventListener('click', (e) => {
    const target = e.target.closest('[data-act]');
    if (!target) return;
    if (target.getAttribute('aria-disabled') === 'true') return;
    handleAction(target.dataset.act, target);
  });

  document.addEventListener('input', (e) => {
    const el = e.target;
    if (el.id === 'palInput') filterPalette();
    if (el.id === 'gate-note' && state.currentId) state.noteDrafts.set(state.currentId, el.value);
    if (el.id === 'composerText' && state.thread) {
      state.drafts.set(state.thread, el.value);
      if (state.sendError) {
        state.sendError = null;
        state.lastSignature = '';
      }
      autosizeText(el);
      const send = $('[data-act="send-message"]');
      if (send) send.disabled = !el.value.trim() || !!composerReason(currentThread()) || state.sending;
    }
  });

  document.addEventListener('change', (e) => {
    const input = e.target;
    if (input.id === 'runSelect') {
      if (input.value === '__new') {
        renderRunSelect();
        state.opener = input;
        newRunError('');
        openDialog('newrun');
        return;
      }
      selectRun(input.value);
      return;
    }
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
      saveSettings({ [setting]: Number(digits) });
    }
  });

  document.addEventListener('submit', (e) => {
    e.preventDefault();
    if (e.target.id === 'composer') sendMessage();
  });

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
    const inComposer = e.target?.id === 'composerText';

    if (inComposer && (e.metaKey || e.ctrlKey) && key === 'enter') {
      e.preventDefault();
      sendMessage();
      return;
    }
    if (e.target?.classList?.contains('resizer') && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
      e.preventDefault();
      nudgeResize(e.target, e.key, e.shiftKey);
      return;
    }
    if ((e.metaKey || e.ctrlKey) && !dialog && (key === 'b' || e.key === '.')) {
      e.preventDefault();
      if (key === 'b') {
        layout.railOpen = !layout.railOpen;
        saveLayout();
        applyLayout();
      } else {
        state.rightOpen = !state.rightOpen;
        layout.factsOpen = state.rightOpen;
        saveLayout();
        renderChrome();
      }
      return;
    }
    if ((e.metaKey || e.ctrlKey) && ['1', '2', '3'].includes(e.key) && !dialog && state.detail) {
      e.preventDefault();
      switchTab({ 1: 'thread', 2: 'diff', 3: 'board' }[e.key]);
      return;
    }
    if (!dialog && (e.key === 'ArrowDown' || e.key === 'ArrowUp') && e.target.closest) {
      const list = e.target.closest('.dv-list, .rail-body');
      if (list) {
        e.preventDefault();
        moveInList(list, e.target, e.key === 'ArrowDown' ? 1 : -1);
        return;
      }
    }
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

    if (e.key === 'Escape' && state.agentMenu) {
      state.agentMenu = false;
      renderView();
      return;
    }
    if (e.key === 'Escape' && routeName() !== 'thread') {
      e.preventDefault();
      closeDialog();
    }
  });

  window.addEventListener('hashchange', applyRoute);

  // ---- init ------------------------------------------------------------------------------------

  loadLayout();
  state.rightOpen = layout.factsOpen;
  $('#resizeRail').addEventListener('pointerdown', (e) => startResize('rail', e));
  $('#resizeFacts').addEventListener('pointerdown', (e) => startResize('facts', e));
  window.addEventListener('resize', applyLayout);
  renderAll();
  loadAppVersion();
  loadSettings();
  poll({ initial: true });
  setInterval(() => poll(), LIST_EVERY_MS);
  requestAnimationFrame(() => requestAnimationFrame(() => document.documentElement.classList.remove('boot')));
})();
