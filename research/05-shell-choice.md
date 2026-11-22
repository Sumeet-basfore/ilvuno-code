# Desktop UI shell choice — decision report

Research pass `2026-11-22`. Read-only: no toolchain installed, no project scaffolded, no repo modified.
`web_search`/`web_fetch` returned HTTP 402 (no credits), so every number below came from
`crates.io/api/v1`, `registry.npmjs.org`, `api.github.com`, `raw.githubusercontent.com`, and one blog
article fetched with `curl`. Versions and download counts are as of this session; each carries its date.

---

## 0. Recommendation

**Tauri 2.x** (stable `2.12.7`, `2026-11-18`) with the Rust runner as a child/embedded process and the
transcript served to the webview over a **loopback HTTP/SSE endpoint**, not Tauri IPC — the same shape
all three reference apps converged on, and the one that keeps a browser/phone client free.
Memory is *not* the reason to pick it: the measured Electron→Tauri idle delta is **26.5 MiB**
(`59.5` vs `86.0` MiB), ~1 % of the 0.9–7.5 GB the 3–5 agent subprocesses will consume.
**Change it if** the §7 Linux spike reproduces WebKitGTK breakage that we must ship through (xterm.js
WebGL, `WEBKIT_DISABLE_DMABUF_RENDERER` abort, the font-weight-100 bug) — then take Electron, because
the React UI is byte-identical either way and the runner is the same Rust process in both.

---

## 1. Comparison matrix

Cells are evidence, not vibes; the source for each row is in the sections that follow.

| | **Electron 45** | **Tauri 2.12** | **egui 0.37** | **iced 0.14** | **Slint 1.19** | **GPUI 0.2.2** |
|---|---|---|---|---|---|---|
| **Memory (idle, all procs)** | 86.0 MiB measured, empty page, macOS | 59.5 MiB, same page, same run | not published; ~40–80 MB typical claim, **unverified** | not published, **unverified** | not published, **unverified** | not published, **unverified** |
| **Bundle on disk** | 268.7 MiB (asar, macOS) | **7.3 MiB** (LTO release) | single binary, few MB + assets | few MB | few MB | few MB + ~109 deps |
| **Startup** | script 321 ms / first frame **326 ms** | script 309 ms / first frame **377 ms** | fastest class (native, no engine) | fast | fast | fastest (Zed-class) |
| **Dev iteration** | Vite HMR + real Chrome DevTools — best in class | Vite HMR for UI; `cargo build` on Rust change; **WebKit Inspector**, weaker than DevTools; `devtools` is a cargo feature | rebuild+relaunch, no DOM inspector (built-in debug painter) | rebuild+relaunch; in-tree `devtools` crate | rebuild; LSP live preview (not verified here) | `hot-reload` feature (subsecond/dioxus) exists but tied to Zed tooling |
| **Markdown / chat** | `react-markdown 10.1.0` + `shiki 4.6.0`, both mature, both used in production by the refs | same as Electron (same webview DOM) | `egui_commonmark 0.26.0` (2.4 M dl) — renders; no rich copy/selection story verified | **nothing** but `iced_highlighter` (in-tree, thin, built on `two-face`) | **nothing** — no markdown crate exists | **nothing** |
| **Diffs** | `react-diff-view 3.3.3` + `diff 9.0.0`; refs write 1.9k–2.8k prod lines on top | same | `similar 3.2.1` (algorithm only) — **write the view yourself** | algorithm only — write it yourself | algorithm only — write it yourself | algorithm only — write it yourself |
| **Terminal** | `@xterm/xterm 6.0.0` + 8 addons (WebGL, fit, serialize, search, image…) — the thing VS Code uses | same (xterm.js in the webview) | `alacritty_terminal 0.26` parser; `egui_term` is at **4,750 downloads, last release 2025-04** | `alacritty_terminal`; `iced_term 0.8` at 27.8 k dl | nothing | nothing |
| **Tables / virtualization** | `@tanstack/react-virtual 3.14.28` (used by orca, cloudroom) | same | `egui_extras::TableBuilder`, `egui_table 0.11`, `egui_ltreeview` (tree) | **no virtualized list widget**; `Lazy` from iced-rs only | `ListView` + model (built in) | Zed's `uniform_list` — not a public API |
| **Theming** | CSS — unlimited, one designer can do it | CSS | egui `Style`/`Visuals`; coarse, "egui look" is hard to escape | theme struct, limited | `.slint` styling, good for panels | `Theme` + `rgb()`; Zed-shaped |
| **Packaging / signing** | `electron-builder 26.18.0`, `electron-updater 6.8.11`, `@electron/notarize 3.1.2` — the reference | built-in bundler (`.app/.dmg/.msi/nsis/.deb/.rpm/.AppImage`) + `tauri-plugin-updater 2.14.1`; **signing mandatory, key loss = no more updates** | none — `cargo-bundle 0.12` + `self_update 1.3.3` + `notarytool` by hand | none, same | none, same | none, same |
| **Cross-platform** | Win/mac/Linux, one Chromium everywhere | Win/mac/Linux **+ iOS/Android** (README: iOS 9+, Android 7+); three different engines | Win/mac/Linux; no mobile, no web | Win/mac/Linux; no mobile | Win/mac/Linux + WASM/web + embedded | macOS/Linux/Windows (Zed ships mac+linux) |
| **Accessibility** | Chromium a11y: AT-SPI / VoiceOver / UIA — best | platform webview a11y (WKWebView good, WebKitGTK AT-SPI ok) | `accesskit 0.25.2` compiled in | **none** — 0 `accesskit` entries in `Cargo.lock` | `accesskit` behind optional `accessibility-text` feature | accesskit-based; README has an a11y topic — not verified |
| **Ecosystem maturity** | 124 k★, 8-week major cadence, 3 refs pin **43/44 while 45 shipped** | 113 k★, 122 releases on 2.x, 3.0 alpha in flight | 32.4 k★, `0.37` on `2026-10-15`, breaking release every ~6–8 weeks | 32.1 k★, **0.14 on 2025-12-07**, `master` is `0.15.0-dev` | 24.7 k★, quarterly 1.x minors; royalty-free but **attribution required** | 95.5 k★ (that is Zed, not gpui), 7 releases ever, none in 13 months |
| **12-month risk** | Chromium majors outpace you (refs already 1–2 behind); nothing else moves | 3.0 migration + `webkit2gtk-rs` 3.0 bump land inside the year | constant breaking churn; you are the a11y/markdown/diff author | one release a year means you fork to fix | vendor risk (SixtyFPS), attribution obligation, weak text stack | **not viable** — see §2 |

---

## 2. The chat-rendering problem — what it actually costs

This is the deciding evidence. Measured in the cloned repos at `/tmp/ilvuno-research` in this session,
`**/*.tsx|*.ts` under the transcript component root, split into total and **prod-only** (tests,
`*.stories.tsx`, `__tests__` excluded):

| Repo | Surface directory | Total | **Prod** | Largest prod file |
|---|---|---|---|---|
| t3code | `apps/web/src/components/chat` | 90,702 | **67,391** | `ChatComposer.tsx` 4,116 · `MessagesTimeline.logic.ts` 2,949 · `MessagesTimeline.tsx` 1,936 |
| cloudroom | `gui/apps/app/src/components/thread` | 53,972 | **37,577** | `ThreadTimelineRows.tsx` 2,612 · `ThreadDetailView.tsx` **3,355** (`03-cloudroom.md` §13) · `build-thread-timeline.ts` 1,726 |
| orca | `src/renderer/src/components/native-chat` | 99,871 | **98,382** | deliberately fragmented: 860 files, largest 450 lines |
| orca | `src/renderer/src/components/terminal-pane` | 81,078 | **79,363** | 772 files |
| cloudroom | `gui/apps/app/src/components/git-diff` | 3,023 | **2,750** | — |
| t3code | `apps/web/src/components/diffs` | 2,830 | **1,857** | — |

Adjacent surfaces that are *not* optional once you have a transcript: cloudroom `promptbox` 37,658 lines
and `secondary-panel` 34,772; t3code `settings` 86,131. None of these numbers include the Rust/Node
runner.

What those lines buy, from the dependency lists actually in each `package.json`:

- orca: `@xterm/xterm` + `addon-webgl/fit/serialize/search/image/ligatures/web-links` + `@xterm/headless`,
  `shiki`, `react-markdown`, `@tanstack/react-virtual`, `@monaco-editor/react`, `diff`.
- cloudroom: `monaco-editor`, `react-markdown`, `@tanstack/react-virtual`.
- t3code: `diff` (its own `ChatMarkdown.tsx` + `MessagesTimeline` logic on top).

**Read this as: nobody in this space writes their own renderer.** Three independent teams, three
licences, same shopping list. The 37 k–98 k prod lines are not markdown rendering — they are the
*behaviour* around it: streaming deltas into a stable layout without re-mounting highlighters, tool-call
rows that expand into diffs, per-agent message shapes, scroll anchoring while tokens arrive, selection
across virtualized rows, and the composer.

### Doing it in Rust instead — what exists, verified

| Need | Crate | Downloads | Last release | Verdict |
|---|---|---|---|---|
| markdown → events | `pulldown-cmark 0.13.6` | 217.1 M | 2026-10-30 | solid, but events only |
| markdown → HTML | `comrak 1.1.0` | 10.8 M | 2026-11-20 | solid; useless without an HTML view |
| markdown widget | `egui_commonmark 0.26.0` | 2.4 M | 2026-10-15 | the **only** maintained one; egui-only |
| syntax highlight | `syntect 5.3.0` / `two-face 0.5.2` | 43.1 M / 18.5 M | 2025-09-27 / 2026-08-07 | mature engine, you own the layout |
| syntax highlight (iced) | `iced_highlighter` (in-tree) | n/a | tracks master | thin, egui-equivalent absent |
| diff algorithm | `similar 3.2.1` / `imara-diff 0.2.0` | 259.8 M / 36.8 M | 2026-10-24 / 2025-06-14 | algorithms, **no view** |
| diff view | `vix-diff-view 2.1.0` | **404** | — | the entire Rust diff-view category |
| terminal parser | `alacritty_terminal 0.26.0` | 3.0 M | 2026-04-06 | real, published, but parser+grid only |
| terminal widget (egui) | `egui_term 0.1.0` | **4,750** | **2025-04-24** | effectively abandoned |
| terminal widget (iced) | `iced_term 0.8.0` | 27.8 k | 2026-03-27 | small, one maintainer |
| code editor widget | `egui_code_editor 0.5.0` | 104 k | 2026-11-03 | highlights, no LSP-grade editing |
| code editor widget | `iced_code_editor 0.7.1` | **4,942** | 2026-11-18 | hobby scale |
| table | `egui_extras 0.37.0` / `egui_table 0.11.0` | 14.1 M / 1.7 M | 2026-10-15 / 2026-10-19 | works; rows must be cheap |
| table (iced) | `iced_table 0.14.0` | 13 k | 2026-05-19 | nothing virtualized in core |

**GPUI is not a candidate for a third party today**, and this is checkable rather than opinion:
`0.2.2` published `2025-10-22`; 7 versions ever; **nothing in 13 months**; 109 crate dependencies
(`blade-graphics`, `metal`, `cocoa`, `objc2`, `naga`…); its own README says "still pre-1.0 … will often be
breaking changes between versions" and then instructs `gpui = { version = "*" }` plus a `gpui_platform`
crate that **is not on crates.io** (`/api/v1/crates/gpui_platform` → not published). You would be
vendoring Zed's workspace and tracking `main`. As a reference design it is excellent; as a dependency it
does not exist yet.

---

## 3. Terminal emulation

Four real options, all verified:

1. **xterm.js in a webview** — `@xterm/xterm 6.0.0` (`2025-12-22`), 21.4 k★, pushed `2026-11-22`, 344 open
   issues, addons for WebGL rendering, fit, search, serialize, images, ligatures, plus `@xterm/headless`
   for server-side scraping. This is what VS Code, orca and cloudroom ship. Nothing in Rust is close in
   completeness: scrollback, selection, mouse modes, IME, hyperlinks, bracketed paste, resize/reflow.
2. **`alacritty_terminal 0.26.0`** — the real parser/grid, published and used by both egui_term and
   iced_term. There is no `wezterm-term` on crates.io (404); wezterm's `termwiz 0.23.3` exists but last
   released `2025-03-20`. `vt100 0.18.1` (21.4 M dl) parses but is not an emulator front-end.
3. **Roll your own on `alacritty_terminal`** — you supply glyph atlas + wgpu/canvas drawing, cell
   damage tracking, selection, mouse reporting, key encoding, IME, search, and reflow. orca's
   `terminal-pane` is **79,363 prod lines** and that is with xterm.js doing the emulation; it is spending
   those lines on PTY transport, remote runtime, tab/session state and UI. A native pane is a
   multi-month item, not a week.
4. **Embed a separate terminal app** — loses the point.

Practical choice: with a webview the terminal is a solved, boring dependency. With any native toolkit it
becomes one of the two or three hardest parts of the product — for a developer tool, that is the worst
place to spend novelty budget.

---

## 4. Packaging and distribution

| | Signing / notarization | Auto-update |
|---|---|---|
| **Tauri** | macOS notarization documented; Windows NSIS/MSI via `signtool`; Linux `.deb/.rpm/.AppImage`. Bundler is part of `tauri-cli 2.12.6`. | `tauri-plugin-updater 2.14.1` (16.1 M dl), **signature verification cannot be disabled** — you must hold the private key; losing it means you can never update installed users again. Endpoint can be static JSON or a dynamic server. |
| **Electron** | `@electron/notarize 3.1.2` + `electron-builder 26.18.0` does the whole matrix (dmg/zip/nsis/AppImage/deb/snap, universal mac). | `electron-updater 6.8.11`: delta updates, channels, staged rollout. The most battle-tested of the six. |
| **egui / iced / Slint / GPUI** | No bundler, no notarization, no updater in-framework. `cargo-bundle 0.12.0` (193 k dl) builds a `.app`; `cargo-dist 0.34.0` ships installers; `self_update 1.3.3` gives an updater you wire yourself, with no code-signing enforcement model. | Assemble it yourself, including the signing chain and the update-manifest format. |

Two Linux notes that matter for a developer tool: Tauri's Linux prerequisites require
`libwebkit2gtk-4.1-dev` (Ubuntu 22.04+) and therefore **the library is a host dependency**, and
`webkit2gtk-rs` is mid-migration to v3.0 (PR #16612, `2026-11-22`), i.e. a dependency bump is coming
whether or not you want it.

---

## 5. Cost to build the v1 UI

Order of magnitude, derived from the prod line counts above plus the crate inventory. "v1" means: one
transcript per thread, streaming, markdown + code highlight, unified and side-by-side diff, file tree,
terminal pane, context ledger — no plugins, no settings matrix.

| Option | Lines we write ourselves | Calendar (1 senior) | What we must build that nobody sells us |
|---|---|---|---|
| **Tauri** | **15 k–25 k** TSX/CSS, plus the Rust runner | **6–10 weeks** to a usable v1 | transcript row model, scroll anchoring, diff wrapper, ledger panel. Markdown, highlighting, diffs, terminal, virtualization all bought. |
| **Electron** | same **15 k–25 k**, same components | **6–10 weeks** | identical to Tauri — strictly a superset cost (bigger binary, Chromium majors) |
| **egui** | **45 k–70 k** | **4–6 months** | markdown layout + link handling, inline code/code blocks with measured line widths, text selection across widgets, clipboard of rich content, diff view, terminal pane, drag-select rows, theming beyond egui's look |
| **iced** | **60 k–90 k** | **6–9 months** | all of egui's list, plus virtualization, plus accessibility (no accesskit at all), against a toolkit that shipped once in the last 12 months |
| **Slint** | **35 k–55 k** | **3–5 months** | markdown/rich-text view (no crate exists), diff view, terminal pane; UI *panels* are the pleasant part, the transcript is the expensive part |
| **GPUI** | unbounded | not planable | the framework itself, since `0.2.2` will not compile from crates.io as documented |

The asymmetry is the whole argument: 37–98 k prod lines exist in the reference apps *because* the web
stack makes the hard parts additive. In a native toolkit the same features are subtractive — each one
removes weeks and adds permanent ownership.

---

## 6. Risks and unknowns

1. **Linux is the actual risk, not memory.** Three verified WebKitGTK facts, each dated:
   `tauri#14286` (open, `2025-10-12`) — font weight is offset by exactly 100 on WebKitGTK, text renders
   bolder than in any browser, CSS computed values are correct; `tauri#16226` (open PR, `2026-10-07`) —
   on Wayland with NVIDIA proprietary drivers or compositors without explicit sync, webkit2gtk 2.44
   enables the DMA-BUF renderer and **GDK calls `abort()`**, which "no Rust error handling or panic
   catching can survive"; only mitigation is `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Third: `devtools` is a
   cargo feature, so release builds ship without an inspector unless you opt in.
2. **Tauri startup is not faster than Electron** in the one controlled measurement: first frame 377 ms
   vs 326 ms. And in 4 of 50 Tauri launches the first frame was held **8–80 s** after the script ran,
   because the window started hidden — a real hazard for tray/restore flows.
3. **Tauri IPC is not the transport for a token stream.** The same benchmark: 50 MiB in 112 ms via
   `ipc::Response`, 279 ms via serde, and a `Channel` resolves `invoke()` *before* its chunks arrive
   (0 chunks received in all 9 calls). Batch deltas, or use loopback HTTP/SSE — which is what gives us
   browser/phone reuse for free.
4. **Tauri 3.0 lands inside our horizon** (alpha.6 `2026-11-06`; milestone "3.0" has 11 open issues;
   "2.13" has 14). Budget one migration. 2.x will stay supported but the ecosystem will fork attention.
5. **Electron's hidden cost is the 8-week Chromium major**, and the refs demonstrate it: orca pins
   `43.8.10`, t3code `44.10.1`, cloudroom `44.11.2`, while `45.4.1` is current. Three teams, all behind.
6. **Attribute/reuse trap.** Slint's royalty-free licence requires shipping `AboutSlint` in an About
   dialog or a "Made with Slint" badge on the download page; the licence also forbids exposing the
   toolkit's APIs. Fine for us, but it is a permanent product constraint, and GPLv3-or-commercial
   otherwise.
7. **Unverified but load-bearing:** whether xterm.js's WebGL addon works acceptably under WebKitGTK
   (canvas fallback exists, but the terminal is where developers notice latency); whether egui's text
   selection and IME behave well enough for a chat composer. Both are §7 tests, not reading.

---

## 7. A 30-minute spike that would settle it

One page, one binary, on the machine we actually ship to (Ubuntu 24.04, Wayland, and the NVIDIA laptop
if that is a target; repeat on macOS). Do not build a chat app — build the four primitives.

1. `npm create tauri-app@latest` (vanilla TS), 2 min.
2. One page, four panes: **(a)** a fake transcript that appends a 60-char markdown delta every 16 ms for
   20 s, with shiki highlighting a code block that is re-highlighted each frame; **(b)** `xterm.js` +
   `addon-webgl` running `top` for 10 s from a Rust `Child`; **(c)** a 2,000-line side-by-side diff from
   `react-diff-view`; **(d)** a 10,000-row virtualized list from `@tanstack/react-virtual`, scrolling.
3. Measure with `ps -o rss= --ppid` over the process tree every second, and `performance.now()` deltas
   into a log; print `p50/p95` frame time.

**Pass criteria (all must hold):** total RSS < 400 MB with 3 idle `node` children running; p95 frame time
< 20 ms during (a) and (d); xterm WebGL context created and key echo latency < 50 ms; no
`GDK` protocol-error abort across 10 launches; text weight matches the same HTML in Firefox within one
screenshot. **Fail any of (b) or the launch test → Electron, same day, same React code, zero UI rewrite.**

Add 10 minutes: repeat step 3 with `node` × 3 at 1 GB each, to confirm the shell's RSS does not scale
with agent memory (it should not; if it does, our process model is wrong, not the shell).

---

## 8. What I could not verify

- **Memory for an app of our shape.** The only controlled Electron↔Tauri measurement I found is an empty
  page plus a JSON payload trickle (Woliński, `2026-11-22`, Tauri 2.12.5 vs Electron 45.2.3, macOS 27.2,
  M1 Pro 32 GB). No published measurement covers a chat app with a real React bundle, highlighters and a
  terminal — and none covers Linux or Windows (the author says so explicitly). The 26.5 MiB delta is
  therefore a *floor*, not our number.
- **Any Linux/Windows memory figure** for either shell. Nothing citable exists that I could reach.
- **xterm.js under WebKitGTK**, and whether the WebGL addon survives the DMA-BUF path.
- **egui / iced / Slint text selection, IME, and copy-with-formatting semantics** — not verifiable by
  reading; needs hands on a build, which this pass was not allowed to do.
- **Whether `gpui 0.2.2` actually builds from crates.io** (`gpui_platform` missing suggests not). I did
  not compile it.
- **Whether Tauri's `.AppImage`/`.deb` bundles carry `libwebkit2gtk-4.1` or require it on the host.** The
  prerequisites doc requires it for *development*; packaging behaviour is unconfirmed.
- **Slint's rich-text ceiling** — no markdown crate exists, but I did not test how far `Text` with spans
  gets us.
- **Real LOC for our own v1.** §5 is an order-of-magnitude estimate scaled from the reference repos, not
  a bottom-up plan.

### Sources (all fetched `2026-11-22` with `curl`)

`crates.io/api/v1/crates/{tauri,egui,iced,slint,gpui,alacritty_terminal,vt100,avt,termwiz,pulldown-cmark,comrak,syntect,two-face,similar,imara-diff,vix-diff-view,egui_commonmark,egui_term,iced_term,iced_table,egui_table,egui_ltreeview,cargo-bundle,self_update,cargo-dist}` ·
`registry.npmjs.org/{@xterm/xterm,@xterm/addon-*,shiki,react-markdown,react-diff-view,diff,@tanstack/react-virtual,monaco-editor,electron,electron-builder,electron-updater,@electron/notarize,@tauri-apps/api}` ·
`api.github.com/repos/{emilk/egui,iced-rs/iced,slint-ui/slint,zed-industries/zed,tauri-apps/tauri,xtermjs/xterm.js}` ·
`api.github.com/repos/tauri-apps/tauri/issues/{14286,16226}` · `api.github.com/repos/tauri-apps/tauri/milestones` ·
`raw.githubusercontent.com/tauri-apps/tauri/{dev/README.md,dev/crates/tauri/Cargo.toml}` ·
`raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/{README.md,Cargo.toml}` ·
`raw.githubusercontent.com/{iced-rs/iced/master/{Cargo.toml,Cargo.lock},iced-rs/iced/master/highlighter/Cargo.toml}` ·
`raw.githubusercontent.com/slint-ui/slint/master/{LICENSE.md,LICENSES/LicenseRef-Slint-Royalty-free-2.0.md}` ·
`raw.githubusercontent.com/tauri-apps/tauri-docs/v2/src/content/docs/{start/prerequisites.mdx,plugin/updater.mdx}` ·
`wolinski.com/blog/tauri-vs-electron-ipc-memory.html` (with `github.com/michalwolinski/tauri-vs-electron-ipc-memory`) ·
local clones `/tmp/ilvuno-research/{t3code,orca,cloudroom}` (line counts computed here, this session).
