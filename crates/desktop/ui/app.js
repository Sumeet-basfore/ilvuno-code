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
  const compact = (n) => {
    const v = Number(n) || 0;
    if (v < 10000) return fmt(v);
    return Math.round(v / 1000) + 'k';
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
    conn: 'connecting', // open | reconnecting | offline | connecting
    runs: [],
    runsError: '',
    currentId: null,
    detail: null,
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
    state.detail = detail;
    state.board = board?.markdown || '';
    pickThread();
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
      renderChrome();
      scheduleRefresh();
    };
    es.onerror = () => {
      state.conn = state.health === false ? 'offline' : 'reconnecting';
      renderChrome();
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

  const enc = (v) => encodeURIComponent(String(v));

  function routeName() {
    const h = hashText();
    if (h.startsWith('settings')) return 'settings';
    if (h === 'palette' || h === 'newrun' || h === 'board') return h;
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
    if (state.view === 'board') return 'board';
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
    if (name === 'board') {
      state.view = 'board';
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
    const parts = String(path || '').split('/').filter(Boolean);
    if (parts.length <= keep) return parts.join('/');
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
    return group === 'needs' ? 'warn' : group === 'working' ? 'ok' : '';
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
    const text = task ? String(task.prompt || '').replace(/\s+/g, ' ').trim().slice(0, 40) : 'No task yet';
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
      statusChip(run.status),
      chip(`${roundsUsed()}/${budget.rounds ?? '—'} rounds · ${compact(run.tokens_used)}/${budget.tokens ? compact(budget.tokens) : '—'} tokens`, 'num'),
      atGate ? `<button class="meta-btn" data-act="focus-gate">${ic('check', 'ic sm')}Gate open</button>` : '',
    ].join('');
  }

  // ---- centre: thread view ---------------------------------------------------------------------

  function scopeChips(scope) {
    if (!scope || !scope.length) return '';
    const items = scope.map((p) => p === 'run'
      ? '<span class="fchip run" title="the whole run">whole run</span>'
      : `<span class="fchip" title="${esc(relPath(p))}">${esc(relPath(p))}</span>`).join('');
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
    const scope = (task.scope || []).length ? ` · ${task.scope.length === 1 && task.scope[0] === 'run' ? 'whole run' : task.scope.length + ' files'}` : '';
    return `
      <div class="ask">
        <button class="ask-card" data-act="task" data-i="${task.i}" aria-label="Open task ${task.i + 1} on ${esc(task.thread)}">
          <div class="ask-meta">round ${esc(task.round ?? '—')} · by ${esc(task.created_by || 'runner')}${esc(scope)}</div>
          <div class="ask-text">${esc(prompt)}</div>
        </button>
      </div>`;
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
          <span class="fchip" title="${esc(a.path)}">${esc(String(a.path || '').split('/').pop())}${a.bytes != null ? ' · ' + fmt(a.bytes) + ' b' : ''}</span>`).join('')}</div>` : ''}
      </div>`;
  }

  function runStopBanner() {
    const run = state.detail.run;
    if (!['blocked', 'budget_exhausted', 'no_progress', 'human_paused'].includes(run.status)) return '';
    const reason = run.blocked_reason || 'The run stopped and nothing was merged.';
    return `
      <section class="runstop" role="alert">
        <div class="runstop-head">${ic('alert-triangle')}<span>Run ${esc(statusWord(run.status))}</span></div>
        <p>${esc(reason)}</p>
        <div class="runstop-actions">
          <button class="btn" data-act="resume">Resume run</button>
          <span class="runstop-meta">${fmt(run.tokens_used)} tokens · ${roundsUsed()} rounds</span>
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
          <h1 class="th-name">${esc(thread.name)}<span class="th-role">${esc(thread.role)}</span></h1>
          <span class="th-agent">${esc(thread.agent)}</span>
          ${safetyChip(thread.agent)}
          ${status}
        </div>
        <p class="th-sub">${thread.tasks.length} ${thread.tasks.length === 1 ? 'task' : 'tasks'} in this run${task ? ` · last in round ${esc(task.round ?? '—')}` : ''}</p>
      </div>
      ${runStopBanner()}
      <div class="conv" role="log" aria-label="Conversation with ${esc(thread.name)}">
        ${thread.tasks.length
          ? thread.tasks.map((t) => `<article class="ex">${askHtml(t)}${replyHtml(t)}</article>`).join('')
          : `<p class="turn-none">No work yet. ${esc(thread.name)} has no task in this run. The box below starts its first one, as a message.</p>`}
      </div>`;
  }

  // ---- centre: board view ----------------------------------------------------------------------

  function ledgerHtml(e, { clamp = true, prefix = 'b' } = {}) {
    const kind = ['intent', 'finding', 'question', 'decision', 'constraint', 'answer'].includes(e.kind) ? e.kind : '';
    const id = prefix + 'led-' + e.id;
    const expanded = state.expanded.has(String(id));
    return `
      <article class="led">
        <div><span class="kind ${kind}">${esc(e.kind || 'entry')}</span></div>
        <div>
          <div class="led-meta">
            <span class="who">by ${esc(e.author || e.thread || 'runner')}</span>
            <span class="sep"> · </span><span>round ${esc(e.round ?? '—')}</span>
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
        ${g.entries.map((e) => ledgerHtml(e, { clamp: false })).join('')}
      </section>`).join('');
  }

  function boardViewHtml() {
    const d = state.detail;
    const ledger = d?.ledger || [];
    return `
      <div class="bview-head">
        <button class="backlink" data-act="back-thread">${ic('chevron-left', 'ic sm')}Back to conversation</button>
        <div class="bview-line">
          <h1>Board</h1>
          <span class="cnt">${ledger.length} ${ledger.length === 1 ? 'entry' : 'entries'} · ${esc(runTitle())}</span>
        </div>
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
          <p class="empty-note">POST /runs on 127.0.0.1:4737</p>
        </div>`;
    }
    return '<p class="facts-empty" style="padding:24px">Reading the run…</p>';
  }

  // ---- right rail ------------------------------------------------------------------------------

  function checksHtml() {
    const rounds = state.detail?.rounds || [];
    if (!rounds.length) return '<p class="facts-empty">No rounds recorded.</p>';
    return rounds.map((r) => {
      const checks = r.readiness?.checks || [];
      // A round with a failed task never runs its checks, so `failing` is the only reason it has.
      const failing = (r.readiness?.failing || []).map((f) => String(f).replace(/^task-failed:/, 'task failed: '));
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
          </div>`).join('')}
        ${checks.length || failing.length ? '' : '<p class="facts-empty" style="padding:6px 0">No checks in this round.</p>'}`;
    }).join('');
  }

  function gatePanel() {
    const { rounds = [] } = state.detail;
    const round = rounds[rounds.length - 1] || { index: 0, readiness: { checks: [] } };
    const checks = round.readiness?.checks || [];
    const draft = state.noteDrafts.get(state.currentId) || '';
    return `
      <section class="gate-panel" aria-label="Gate">
        <div class="gate-phead"><span class="gate-k">Gate · round ${esc(round.index)}</span>${chip('open', 'warn')}</div>
        <p class="gate-copy">${checks.length ? 'The checks passed.' : 'This round recorded no checks.'} Nothing has been merged. Accept to record the decision, or send it back with a note the team reads.</p>
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
          <button class="btn primary" data-act="accept-gate"${state.gateBusy ? ' disabled' : ''}>Accept and merge</button>
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
        <div class="gate-t"><b>Gate closed · ${verb}</b><br>${verb === 'merged' ? 'Merged' : 'Accepted'} at round ${at} · ${n} of ${checks.length} checks passed</div>
      </div>`;
  }

  function gateSection() {
    const run = state.detail.run;
    if (run.status === 'reviewed') return gatePanel();
    if (run.status === 'accepted' || run.status === 'merged') return gateReceipt();
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
        <p class="facts-empty">${state.health === false ? 'The runner is not answering, so there is nothing to check.' : 'The board fills in as threads report.'}</p>`;
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
      ${runFactsHtml(state.detail.run)}
      <section class="fsec" aria-labelledby="sec-board">
        <div class="fsec-head"><h2 id="sec-board">Board</h2><span class="n">${ledger.length}</span>
          <button class="fsec-link" data-act="board">View all</button></div>
        ${ledger.length ? ledger.slice().reverse().map((e) => ledgerHtml(e)).join('') : '<p class="facts-empty">No board entries yet.</p>'}
      </section>
      <section class="fsec" aria-labelledby="sec-checks">
        <div class="fsec-head"><h2 id="sec-checks">Checks</h2><span class="n">${passed} / ${checks.length}</span></div>
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
      <section class="fsec" aria-labelledby="sec-run">
        <div class="fsec-head"><h2 id="sec-run">Run</h2><span class="n">${esc(statusWord(run.status))}</span></div>
        <div class="frow"><span>Folder</span><span class="v" title="${esc(run.project || '')}">${esc(tailPath(run.project) || '—')}</span></div>
        <div class="frow"><span>Id</span><span class="v" title="${esc(run.id || '')}">${esc(String(run.id || '—').slice(0, 8))}</span></div>
        <div class="frow"><span>Ran for</span><span class="v">${esc(ranFor)}</span></div>
        <div class="frow"><span>Rounds</span><span class="v"><b>${roundsUsed()}</b>${budget.rounds ? ' / ' + esc(budget.rounds) : ''}</span></div>
        <div class="frow"><span>Tokens</span><span class="v"><b>${compact(tokens)}</b>${cap ? ' / ' + compact(cap) : ''}</span></div>
        ${cap ? `<div class="bar${pct >= 90 ? ' hot' : ''}" role="img" aria-label="${pct}% of this run's token budget used"><i style="width:${pct}%"></i></div>` : ''}
      </section>`;
  }

  function renderFacts() {
    const body = $('#factsBody');
    if (!body) return;
    const top = body.scrollTop;
    body.innerHTML = factsHtml();
    if (state.lastRunId === state.currentId) body.scrollTop = top;
    else body.scrollTop = 0;
    $('#factsLbl').textContent = state.detail ? 'Run work' : 'No run';
  }

  // ---- centre render ---------------------------------------------------------------------------

  function renderView() {
    const view = $('#view');
    const scroll = $('#scroll');
    if (!view || !scroll) return;
    const top = scroll.scrollTop;
    const key = `${state.currentId}|${state.view}|${state.thread}`;
    const same = key === state.lastThreadKey;

    if (!state.detail) {
      view.className = 'view';
      view.innerHTML = emptyState();
      scroll.scrollTop = 0;
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
    if (run.status !== 'working') return `This run is ${statusWord(run.status)}. Start a new run to continue.`;
    return '';
  }

  function autosizeText(el) {
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 150) + 'px';
  }

  function renderComposer() {
    const form = $('#composer');
    const thread = currentThread();
    const show = !!state.detail && state.view === 'thread' && !!thread;
    form.hidden = !show;
    if (!show) return;

    const text = $('#composerText');
    const send = $('[data-act="send-message"]');
    const note = $('#composerNote');
    const error = $('#composerError');
    const draft = state.drafts.get(thread.name) || '';
    if (text.value !== draft) text.value = draft;
    autosizeText(text);

    const reason = composerReason(thread);
    const usable = !reason && !state.sending;
    text.disabled = !usable;
    text.placeholder = `Message ${thread.name}…`;
    send.disabled = !usable || !draft.trim();
    form.querySelector('.composer-box').classList.toggle('is-off', !usable);
    note.textContent = reason || (thread.tasks.length
      ? `Ctrl+Enter sends. It arrives as a message task on ${thread.name}.`
      : `${thread.name} has no task yet. Sending starts its first one, as a message.`);

    const err = state.sendError && state.sendError.thread === thread.name ? state.sendError.message : '';
    error.hidden = !err;
    error.textContent = err;
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

    const app = $('#app');
    app.dataset.right = state.rightOpen ? 'open' : '';
    const toggle = $('.facts-toggle');
    toggle.setAttribute('aria-expanded', String(state.rightOpen));

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
        return `<div class="dfile" title="${esc(a.path)}">${ic('folder')}<span>${esc(fn)}</span><span class="sz">${a.bytes != null ? fmt(a.bytes) + ' b' : '—'}</span></div>`;
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
            </div>`).join('') || '<p class="facts-empty">The API reported no agents.</p>'}
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
            <input class="num-input" type="number" min="1" max="20" value="${esc(s.budget_rounds)}" data-setting="budget_rounds" aria-label="Default rounds">
          </div>
          <div class="form-row">
            <div class="form-label">Tokens<small>A soft stop. The round in flight finishes; no new round starts.</small></div>
            <input class="num-input" type="text" inputmode="numeric" value="${esc(fmt(s.budget_tokens))}" data-setting="budget_tokens" aria-label="Default tokens">
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
        <div class="k">Run</div>
        <div class="kv"><span>Run id</span><span class="v">${esc(state.currentId || '—')}</span></div>
        <div class="kv"><span>Threads</span><span class="v">${state.detail?.threads?.length || 0}</span></div>
      </div>
      <div class="set-sec">
        <div class="k">Keyboard</div>
        <div class="kv"><span>Command palette</span><span class="keys"><span class="kbd">Ctrl K</span></span></div>
        <div class="kv"><span>New run</span><span class="keys"><span class="kbd">Ctrl N</span></span></div>
        <div class="kv"><span>Send a message</span><span class="keys"><span class="kbd">Ctrl ↵</span></span></div>
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
    return { act: d.act, name: d.name, i: d.i, id: d.id, tab: d.tab, target: d.target };
  }

  function restoreFocus(token) {
    if (!token) return;
    const all = $$(`[data-act="${token.act}"]`);
    const el = all.find((n) => ['name', 'i', 'id', 'tab', 'target']
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
    });
  }

  let lastSignature = '';

  function renderAll(force = false) {
    const signature = viewSignature();
    if (!force && signature === lastSignature) return;
    const focused = focusToken();

    renderRail();
    renderView();
    renderComposer();
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
        state.rightOpen = !state.rightOpen;
        renderChrome();
        break;
      }
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
      case 'toggle-gate': {
        if (!state.settings) return;
        const next = !state.settings.human_gate;
        target.setAttribute('aria-checked', String(next));
        saveSettings({ human_gate: next });
        break;
      }
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

    if (e.key === 'Escape' && routeName() !== 'thread') {
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
