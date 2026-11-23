// The Ilvuno window. Reads the local API (docs/api.md) and renders it. No data is invented: every number,
// name and sentence comes from a response. Agent text is shown with textContent, never as HTML.

const API = "http://127.0.0.1:4737";
const LIST_EVERY_MS = 5000;
const RELOAD_AFTER_EVENT_MS = 300;
const HEALTH_TIMEOUT_MS = 2000;
const DOWN_RETRY_MS = 3000;
const TRUNCATE_PROMPT_AT = 240;
const START_RUN_CMD = "RUSTUP_TOOLCHAIN=1.100.0 cargo run -q -p cli -- run --goal examples/board-experiment.json";
const START_RUNNER_CMD = "RUSTUP_TOOLCHAIN=1.100.0 cargo run -p api --bin ilvunod";
const GOOD = new Set(["accepted", "merged", "pass", "reported"]);
const BAD = new Set(["blocked", "budget_exhausted", "no_progress", "block", "failed"]);

// Kept across re-renders. The page is rebuilt on every journal event, so anything the person opened,
// typed or loaded must live here, keyed by run id, not in the DOM.
const openFolds = new Set(); // "runId|key" for each <details> the person has open
const boardText = new Map(); // runId -> last board markdown
const noteDrafts = new Map(); // runId -> the send-back note being typed

class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

// ---- small DOM helpers -------------------------------------------------------------------------

function el(tag, props, ...kids) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props || {})) {
    if (value === null || value === undefined || value === false) continue;
    if (key === "class") node.className = value;
    else if (key.startsWith("on")) node.addEventListener(key.slice(2), value);
    else node.setAttribute(key, value === true ? "" : String(value));
  }
  for (const kid of kids.flat()) {
    if (kid === null || kid === undefined || kid === false) continue;
    node.append(kid instanceof Node ? kid : document.createTextNode(String(kid)));
  }
  return node;
}

function chip(status) {
  const kind = GOOD.has(status) ? "pass" : BAD.has(status) ? "fail" : "";
  return el("span", { class: `chip ${kind}`.trim() }, status);
}

// A <details> whose open state survives re-renders. `key` is unique within the run.
function fold(runId, key, summary, ...kids) {
  const id = `${runId}|${key}`;
  const node = el("details", { "data-key": key, open: openFolds.has(id) }, el("summary", {}, summary), ...kids);
  node.addEventListener("toggle", () => {
    if (node.open) {
      openFolds.add(id);
      if (key === "board") refreshBoard(runId);
    } else {
      openFolds.delete(id);
    }
  });
  return node;
}

// The top-bar mark for the run page's event stream. `null` hides it, on pages with no stream.
const liveMark = {
  node: el("span", { class: "live off" }, "offline"),
  set(on) {
    this.node.hidden = on === null;
    this.node.className = on ? "live" : "live off";
    this.node.textContent = on ? "live" : "offline";
  },
};

function shell(...children) {
  const app = document.getElementById("app");
  const top = el("nav", { class: "app-top" }, el("a", { href: "#/runs" }, "Ilvuno · runs"), liveMark.node);
  app.replaceChildren(top, ...children.flat());
}

// ---- the API ----------------------------------------------------------------------------------

async function fetchWithTimeout(url, ms) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), ms);
  try {
    return await fetch(url, { signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

async function readJson(response) {
  const body = await response.json().catch(() => null);
  if (!response.ok) throw new ApiError(response.status, body?.error || `HTTP ${response.status}`);
  return body;
}

async function getJson(path) {
  return readJson(await fetch(API + path, { headers: { accept: "application/json" } }));
}

async function postJson(path, data) {
  return readJson(
    await fetch(API + path, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(data),
    }),
  );
}

// True only when /health answers 200. A refused connection or a timeout counts as "not running".
async function runnerIsUp() {
  try {
    return (await fetchWithTimeout(API + "/health", HEALTH_TIMEOUT_MS)).ok;
  } catch {
    return false;
  }
}

// ---- routing -----------------------------------------------------------------------------------

let stopView = null;

function route() {
  clearTimeout(downRetry);
  if (stopView) {
    stopView();
    stopView = null;
  }
  const match = location.hash.match(/^#\/runs\/([^/]+)$/);
  if (!match) return listView();
  let id;
  try {
    id = decodeURIComponent(match[1]);
  } catch {
    return notFoundPage("");
  }
  return runView(id);
}

window.addEventListener("hashchange", route);

// ---- the banner and the empty states ----------------------------------------------------------

// While the runner is down, the page also retries on its own, so a runner started later is picked up.
let downRetry = null;

function runnerDownPage() {
  clearTimeout(downRetry);
  downRetry = setTimeout(route, DOWN_RETRY_MS);
  shell(
    el(
      "section",
      { class: "banner", role: "alert" },
      el("h2", {}, "The runner is not running"),
      el("p", {}, "This window reads the runner on 127.0.0.1:4737. Start it from the project folder:"),
      el("pre", { class: "cmd" }, START_RUNNER_CMD),
      el("p", {}, "Or run tools/window.sh, which starts the runner and this window together."),
      el("button", { class: "primary", onclick: () => route() }, "Retry"),
    ),
  );
}

function notFoundPage(id) {
  shell(
    el(
      "section",
      { class: "notfound" },
      el("h1", {}, "No run has this id"),
      el("p", { class: "muted" }, id ? `There is no run with the id ${id}.` : "The address does not name a run."),
      el("a", { href: "#/runs" }, "← Back to all runs"),
    ),
  );
}

// ---- the list ----------------------------------------------------------------------------------

function listView() {
  liveMark.set(null);
  let alive = true;
  const load = async () => {
    if (!alive) return;
    if (!(await runnerIsUp())) return runnerDownPage();
    try {
      const runs = await getJson("/runs");
      if (alive) renderList(runs);
    } catch {
      if (alive) runnerDownPage();
    }
  };
  load();
  const timer = setInterval(load, LIST_EVERY_MS);
  stopView = () => {
    alive = false;
    clearInterval(timer);
  };
}

function renderList(runs) {
  const header = el("header", { class: "list-head" }, el("h1", {}, "Runs"));
  if (runs.length === 0) {
    return shell(
      header,
      el(
        "section",
        { class: "banner quiet" },
        el("h2", {}, "No runs yet"),
        el("p", {}, "Start one from the project folder. The example goal costs about 12 cents of Claude quota:"),
        el("pre", { class: "cmd" }, START_RUN_CMD),
      ),
    );
  }
  shell(
    header,
    ...runs.map((run) =>
      el(
        "a",
        { class: "runrow", href: `#/runs/${encodeURIComponent(run.id)}` },
        el("div", { class: "title-line" }, el("h3", {}, run.title || "(no goal text)"), chip(run.status)),
        el("div", { class: "proj" }, run.project),
        el(
          "div",
          { class: "nums" },
          el("span", {}, el("b", {}, run.rounds_used), " rounds"),
          el("span", {}, el("b", {}, formatInt(run.tokens_used)), " tokens"),
          el("span", {}, formatTime(run.started_at)),
        ),
      ),
    ),
  );
}

// ---- one run -----------------------------------------------------------------------------------

function runView(id) {
  liveMark.set(false);
  let alive = true;
  let reloadTimer = null;
  let events = null;

  const stop = () => {
    alive = false;
    clearTimeout(reloadTimer);
    clearInterval(pollTimer);
    if (events) events.close();
  };

  const load = async () => {
    if (!alive) return;
    if (!(await runnerIsUp())) {
      stop();
      return runnerDownPage();
    }
    try {
      const data = await getJson(`/runs/${encodeURIComponent(id)}`);
      if (!alive) return;
      renderRun(id, data);
      if (openFolds.has(`${id}|board`)) refreshBoard(id);
    } catch (err) {
      stop();
      if (err instanceof ApiError && err.status === 404) return notFoundPage(id);
      runnerDownPage();
    }
  };

  const scheduleReload = () => {
    clearTimeout(reloadTimer);
    reloadTimer = setTimeout(load, RELOAD_AFTER_EVENT_MS);
  };

  // Open the stream first, then load: an event that lands between the two is not lost.
  events = new EventSource(`${API}/runs/${encodeURIComponent(id)}/events`);
  events.onopen = () => {
    liveMark.set(true);
    scheduleReload();
  };
  events.onerror = () => liveMark.set(false);
  events.addEventListener("journal", scheduleReload);

  // If the stream drops, this timer still reloads, and a dead runner shows the banner.
  const pollTimer = setInterval(() => {
    if (!events || events.readyState !== EventSource.OPEN) load();
  }, LIST_EVERY_MS);

  stopView = stop;
  load();
}

async function refreshBoard(id) {
  try {
    const { markdown } = await getJson(`/runs/${encodeURIComponent(id)}/board`);
    boardText.set(id, markdown);
  } catch {
    boardText.set(id, "The board could not be read. Try again in a moment.");
  }
  const pre = document.querySelector('#app details[data-key="board"] pre');
  if (pre) pre.textContent = boardText.get(id);
}

function renderRun(id, data) {
  const { run, goal, threads = [], rounds = [], tasks = [], artifacts = [] } = data;
  const sections = [el("div", { class: "app-back" }, el("a", { href: "#/runs" }, "← All runs"))];
  if (run.blocked_reason) {
    sections.push(
      el("div", { class: "reason", role: "status" }, el("b", {}, `Run ${run.status.replace(/_/g, " ")}`), run.blocked_reason),
    );
  }
  sections.push(runHeader(id, run, goal, threads, tasks));
  if (run.status === "reviewed") sections.push(gatePanel(id));
  sections.push(
    el("h2", { class: "sec" }, "What happened, by round"),
    roundDigest(rounds, tasks),
    el("h2", { class: "sec" }, "Tasks, in the order they ran", el("span", {}, `${tasks.length} tasks`)),
    ...tasks.map((task, index) => taskCard(id, task, index)),
    el("h2", { class: "sec" }, "Checks"),
    checksTable(rounds),
    el("h2", { class: "sec" }, "Artifacts"),
    artifactsTable(artifacts),
    boardFold(id),
    el("footer", {}, `Run ${id} · read from 127.0.0.1:4737 at ${new Date().toLocaleTimeString()}`),
  );
  shell(...sections);
}

function runHeader(id, run, goal, threads, tasks) {
  return el(
    "header",
    { class: "run" },
    el("div", { class: "proj" }, run.project),
    el("h1", {}, goal.text),
    el("p", { class: "done" }, goal.done),
    el(
      "div",
      { class: "meta" },
      chip(run.status),
      el("span", {}, el("b", {}, run.rounds_used), " rounds"),
      el("span", {}, el("b", {}, formatInt(run.tokens_used)), " tokens"),
      el("span", {}, el("b", {}, threads.length), " threads"),
      el("span", {}, el("b", {}, tasks.length), " tasks"),
      fold(id, "goal", "full goal", el("pre", {}, goal.text)),
    ),
  );
}

// The gate: the only action the window takes. It appears only while the run waits for a person.
function gatePanel(id) {
  const note = el("textarea", {
    "aria-label": "Note for the send-back",
    placeholder: "What should change? The agents read this note.",
    oninput: () => noteDrafts.set(id, note.value),
  });
  note.value = noteDrafts.get(id) ?? "";
  const message = el("p", { class: "error", role: "alert" });
  const buttons = [];
  const act = async (decision) => {
    message.textContent = "";
    if (decision === "send_back" && note.value.trim() === "") {
      message.textContent = "Write a note first. The agents need to know what to change.";
      return;
    }
    buttons.forEach((b) => (b.disabled = true));
    try {
      const body = decision === "accept" ? { decision } : { decision, note: note.value.trim() };
      await postJson(`/runs/${encodeURIComponent(id)}/gate`, body);
      noteDrafts.delete(id);
      route();
    } catch (err) {
      message.textContent = err instanceof ApiError ? err.message : "The runner could not be reached.";
      buttons.forEach((b) => (b.disabled = false));
    }
  };
  const accept = el("button", { class: "primary", onclick: () => act("accept") }, "Accept");
  const sendBack = el("button", { onclick: () => act("send_back") }, "Send back");
  buttons.push(accept, sendBack);
  return el(
    "section",
    { class: "gate" },
    el("h2", {}, "Gate — waiting for you"),
    el("p", {}, "The run reached its checks. Accept it, or send it back with a note the agents will read."),
    note,
    el("div", { class: "row-actions" }, accept, sendBack),
    message,
  );
}

function roundDigest(rounds, tasks) {
  const indexes = [...new Set([...rounds.map((r) => r.index), ...tasks.map((t) => t.round)])].sort((a, b) => a - b);
  if (indexes.length === 0) return el("p", { class: "empty" }, "No round has started yet.");
  return indexes.map((index) => {
    const round = rounds.find((r) => r.index === index);
    const mine = tasks.filter((t) => t.round === index);
    const threadNames = [...new Set(mine.map((t) => t.thread))].join(", ");
    const failing = (round?.readiness?.checks ?? []).filter((c) => !c.passed).map((c) => c.name);
    let sentence = threadNames;
    if (!round?.readiness) sentence += " — not reviewed yet.";
    else if (failing.length) sentence += ` — failed: ${failing.join("; ")}.`;
    else sentence += " — every check passed.";
    return el(
      "div",
      { class: "round" },
      el("div", { class: "n" }, `round ${index}`),
      el("div", {}, el("h3", {}, `${mine.length} task(s) `, chip(round?.verdict ?? round?.status ?? "open")), el("p", {}, sentence)),
    );
  });
}

function taskCard(runId, task, index) {
  const scope = (task.scope || []).join(", ");
  return el(
    "div",
    { class: "card" },
    el("div", { class: "head" }, el("h3", {}, `${index + 1} · ${task.thread}`), chip(task.status)),
    el("p", { class: "sub" }, truncate(task.prompt, TRUNCATE_PROMPT_AT)),
    el("div", { class: "scope" }, `round ${task.round} · by ${task.created_by} · scope: ${scope || "none"}`),
    task.message ? fold(runId, `task-${index}`, `what the agent said (${task.message.length} chars)`, el("pre", {}, task.message)) : null,
  );
}

function checksTable(rounds) {
  const rows = rounds.flatMap((round) =>
    (round.readiness?.checks ?? []).map((check) =>
      el(
        "tr",
        {},
        el("td", { class: "num" }, `round ${round.index}`),
        el("td", {}, check.name),
        el("td", { class: "num detail" }, check.detail),
        el("td", {}, chip(check.passed ? "pass" : "fail")),
      ),
    ),
  );
  if (rows.length === 0) return el("p", { class: "empty" }, "No checks have been run.");
  return el(
    "div",
    { class: "table-wrap" },
    el(
      "table",
      {},
      el("thead", {}, el("tr", {}, ...["Round", "Check", "Detail", ""].map((h) => el("th", {}, h)))),
      el("tbody", {}, ...rows),
    ),
  );
}

function artifactsTable(artifacts) {
  if (artifacts.length === 0) return el("p", { class: "empty" }, "No artifacts yet.");
  // The window cannot open files, so the paths are shown, not linked.
  return el(
    "div",
    { class: "table-wrap" },
    el(
      "table",
      {},
      el("thead", {}, el("tr", {}, ...["Kind", "Path", "Bytes", "sha256"].map((h) => el("th", {}, h)))),
      el(
        "tbody",
        {},
        ...artifacts.map((a) =>
          el(
            "tr",
            {},
            el("td", {}, a.kind),
            el("td", { class: "scope", title: a.path }, relativeArtifactPath(a.path)),
            el("td", { class: "num" }, formatInt(a.bytes)),
            el("td", { class: "scope" }, String(a.sha256 || "").slice(0, 12)),
          ),
        ),
      ),
    ),
  );
}

// The full board is read only when the fold is open, and re-read on each live change while it is.
function boardFold(id) {
  const pre = el("pre", { class: "board" }, boardText.get(id) ?? "Opening the board…");
  const node = fold(id, "board", "Full board — every entry in order", pre);
  node.style.marginTop = "36px";
  return node;
}

// ---- helpers -----------------------------------------------------------------------------------

function formatInt(n) {
  return Number(n ?? 0).toLocaleString("en-US");
}

function formatTime(ms) {
  return ms ? new Date(ms).toLocaleString() : "";
}

function truncate(text, max) {
  const flat = String(text ?? "");
  return flat.length > max ? `${flat.slice(0, max)}…` : flat;
}

function relativeArtifactPath(path) {
  const text = String(path);
  const at = text.indexOf("/artifacts/");
  return at >= 0 ? text.slice(at + 1) : text;
}

route();
