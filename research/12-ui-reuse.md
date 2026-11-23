# UI reuse report: t3code and orca

For: our Tauri board app (plain HTML, CSS and JS; no React; no bundler).
Scope: read-only. Nothing in the two clones was changed. No installs or builds.
Clones: `/tmp/ilvuno-ref/t3code` (MIT, T3 Tools Inc.) and `/tmp/ilvuno-ref/orca` (MIT, Lovecast Inc.).
Line numbers refer to those clones. Both apps are Electron. Neither has Tauri-specific code.


> Read for stage 0. Clones were shallow copies at `/tmp/ilvuno-ref/{t3code,orca}`; re-clone with
> `git clone --depth 1 https://github.com/pingdotgg/t3code` and the same for `stablyai/orca`.
> One finding is a defect in our own design: `--ink-3` failed contrast. Fixed in both renderers
> the day it was reported.

## Contents

1. Summary
2. How to read the verdicts
3. Licences and flags
4. Run and thread list
5. Transcript
6. Diff view
7. Status and progress
8. Connection state
9. Empty and error copy
10. Keyboard and command palette
11. Design tokens and contrast
12. Composer and task creation
13. Pieces to lift (table)
14. What I did not find
15. Top 5 for our window this week

---

## 1. Summary

- **Orca has a board.** A kanban of agents with four lanes: Needs You, Working, Done, Idle. It is the closest match to our board-first layout. Its rules matter more than its UI.
- **Best rules (orca).** "Done" stays marked until someone opens it. A quiet agent is not called "working". A failure you have opened stops shouting.
- **Best words (t3code).** Status labels, "Worked for 2m" fold labels and error copy are short and clear.
- **Best send model (t3code).** While a turn runs, a click queues the message. A modifier-click steers the running turn.
- **Neither repo has our gate.** Neither has a per-change accept or send-back control.
- **Copy behaviour and strings. Do not copy React components.** Take a handful of small pure files (section 13).
- **Our proto has one accessibility defect.** `--ink-3` (#8a857a) is 3.34:1 on paper. Small text needs 4.5:1. Fix in section 11.2.

## 2. How to read the verdicts

- **adopt**: usable almost as-is in vanilla JS. Keep the licence notice.
- **adapt**: rebuild from the behaviour described. Do not paste the file.
- **skip**: exists, not for us. One reason.
- **Drag**: what a piece pulls in. "none" means no runtime imports I saw.

Drag key: **[R]** React · **[Z]** zustand · **[I]** i18n wrapper · **[D]** design system · **[X]** big library (Monaco, Legend List, @pierre/diffs, Tiptap, TanStack Virtual).

---

## 3. Licences and flags

Both repos are MIT. Most source files have no copyright header. If we copy a file, keep the repo's MIT text and copyright line in our third-party notice.

| # | Where | What | Action |
|---|---|---|---|
| L1 | `t3code/apps/web/deps/pierre-words-…tgz` | Vendored fork of `@pierre/words`. `package.json` says **Apache-2.0**. | Not MIT. Keep LICENSE and NOTICE if used. We do not need it. |
| L2 | `t3code/apps/web/src/terminal/ghostty/vendor/*.wasm` | Two binaries. No licence text in that folder. | Unverified. Do not use. |
| L3 | `t3code/apps/web/src/terminal/ghostty/fonts/LICENSE` | Says MIT for Symbols Nerd Font Mono. orca ships the same font with an OFL file (`assets/fonts/SymbolsNerdFontMono-OFL.txt`). | The two disagree. Check before shipping the font. |
| L4 | `orca/src/renderer/src/assets/text-shimmer.css:1` | Header says shadcn/ui, MIT. | Keep that notice if adapted. |
| L5 | `orca/…/assets/fonts/Geist-Variable.woff2` | Geist. `orca/docs/site/THIRD_PARTY_NOTICES.md`: SIL OFL 1.1. | For v1, do not bundle. If bundled, keep OFL text. |
| L6 | `t3code/apps/web/src/lib/softwareKeyboard.ts:43` | "Adapted from Ionic's keyboard assist, MIT, Drifty Co." | MIT. Mobile-only. Skip. |
| L7 | `orca/src/shared/speech/dictation/dictation-english-words.ts` | Word list with an embedded licence block. | Read the block first. Skip for us. |
| L8 | npm packages (`@pierre/diffs`, `@legendapp/list`, `@tanstack/react-virtual`, `@monaco-editor/react`, Tiptap, zustand, i18next, gt-react) | Not vendored. I did not check their licences here. | Check each package's LICENSE before use. |

---

## 4. Run and thread list

### 4.1 Orca board (closest match)

- **File.** `orca/src/renderer/src/components/dashboard/board/AgentKanbanBoard.tsx` (294 lines).
- **Lanes.** Needs You / Working / Done / Idle (`AgentKanbanBoard.tsx:35–44`). An empty lane says "None" (`en.json`, `dashboardPopout.bucket.empty`).
- **Idle is hidden.** Shown only with "Show idle agents" (`AgentKanbanBoard.tsx:136`). Helper text: "Include agents that have gone quiet for 30 minutes without reporting completion. Hidden by default."
- **Order in a lane.** Newest state change first (`board/agent-board-projection.ts:86`).
- **Card** (`board/AgentKanbanCard.tsx`, 341 lines): started-ago label ("just now", "5m", "2h", "3d", lines 26–43); last user and agent message, clamped to one line (`AgentCardLastMessages.tsx`); subagent count; review pill; repo and worktree name.
- **Bucket map** (`dashboard/snapshot/dashboard-card-bucket.ts:1–19`): working and monitoring → Working; done → Done; idle → Idle; blocked and waiting → Needs You.
- **Done stays green until seen** (`shared/agent-status/presentation/dashboard-snapshot.ts:35–42`). Unseen done stays Done. Seen done becomes Idle.
- **Verdict:** adapt. Drag: [R] [Z] [X]. Keep the sort and bucket rules. Drop the React caching.

### 4.2 Staleness rule (the most useful idea in either repo)

- **Threshold.** 30 minutes (`shared/agent-status/contracts/agent-status-freshness.ts:4`).
- **What happens** (`presentation/agent-row-decay-state.ts`). A working, blocked or waiting agent with no fresh evidence for 30 minutes changes state:
  - live terminal attached → **unverifiable**. The row says "No update in 34m".
  - no terminal → **idle**.
- **Board placement.** Unverifiable is shown as idle on the board (`dashboard/rows/dashboard-row-bucket.ts:14`). It drops into the hidden Idle lane, but its row still says "No update in Xm". Nobody mistakes it for live work.
- **Helpers.** `formatCompactDuration` (`agent-row-decay-state.ts:49–60`) gives `34m`, `2h`, `3d`, rounded down. `agentNoUpdateLabel` (lines 62–66).
- **For us.** Our agents write a journal. A silent run should say "No update in 12m", not "Working".
- **Verdict:** adopt. Drag: none. Two files.

### 4.3 t3code sidebar row

- `apps/web/src/components/sidebar/SidebarThreadRow.tsx` (1968 lines): title, status word, terminal dot, PR status, branch, settle/snooze/pin actions.
- **Verdict:** skip. Drag: [R] [I] [D] [X: dnd-kit] [Z].

### 4.4 t3code status resolver

- **Where.** `apps/web/src/components/Sidebar.logic.ts:964–994`, `resolveSidebarThreadStatus`. First match wins:
  1. pending approval → **approval**
  2. pending user input → **input**
  3. guest waiting on owner → **request**
  4. running, blocked on a child thread → **waiting**
  5. preparing → **preparing**
  6. queued, starting, running or waiting → **working**
  7. idle → **waiting**
  8. failed → **limited** (usage limit) or **failed**
  9. otherwise → **ready**
- **Failure settles** (`resolveSidebarRowStatus`, lines 997–1004). An opened failure becomes "ready".
- **Unseen completion** (`hasUnseenCompletion`, lines 779–791). Unseen if the run completed after the last visit. Interrupted and cancelled never count.
- **Folder dot** (`resolveSidebarFolderDot`, line 1032). A closed group shows its most urgent hidden child. Waiting stays quiet.
- **Verdict:** adapt. Rewrite over our states. The file is 1505 lines; do not copy it.

### 4.5 Sort and snooze (t3code)

- `packages/client-runtime/src/state/threadSort.ts` (287 lines). A thread's time falls back to its last activity (lines 30–56). **Adapt.**
- Snooze with "Woke" on return (`threadActionMenu.logic.ts:176`). **Skip for v1.**

---

## 5. Transcript

### 5.1 Turn fold (t3code)

- `apps/web/src/components/chat/MessagesTimeline.tsx:1888–1924`, `TurnFoldTimelineRow`. A finished turn collapses to one row with a chevron and `aria-expanded`. Contents load on demand, with a skeleton (line 1871).
- **Labels** (`MessagesTimeline.logic.ts:1266–1271`): "Worked for 2m 14s" / "Worked" (no duration) / "You stopped after 2m 14s" / "You stopped this response".
- **Duration format** (`packages/client-runtime/src/state/models.ts:419–430`): "45s", "3m", "1h 5m".
- **For us:** "Round 0 · 3 tasks · 2m 14s". **Adapt.** Drag: [R] [I] [X: Legend List].

### 5.2 Tool-call labels (t3code)

- **Live** (`MessagesTimeline.logic.ts:259–300`): "Running {program}", then "Failed", "Declined", "Stopped" or "Ran {program}". Reasoning shows "Thinking" while live, "Thought" when done.
- **Static** (`:149–214`): first match wins: notice label; thought text (whitespace collapsed); tool display name; command text; "Read file" or "path +N more"; tool title in sentence case.
- **For us:** "Running file_exists brief.html", then "Found brief.html" or "Missing brief.html". **Adapt.**

### 5.3 Reading position (t3code)

- `apps/web/src/lib/preserveReadingPosition.ts` (115 lines). When width changes or rows re-measure, it keeps the reader's text line in place. A wheel or touch move cancels the correction.
- **Dependency.** `lib/observeResize.ts` (96 lines) imports `flushSync` from `react-dom` (lines 1, 44). Replace it with a direct callback.
- **Verdict:** adopt after that one edit. Drag: none after the edit.

### 5.4 Anchoring a sent message (t3code)

- `packages/shared/src/chatList.ts`. `resolveChatListAnchoredEndSpace` returns one item's index plus a 16px offset (`CHAT_LIST_ANCHOR_OFFSET`). The sent message stays near the top while the reply grows below it. **Adapt** if we add "start a task, then watch it". Drag: none seen.

### 5.5 Virtual lists

- **t3code** uses `@legendapp/list` (`MessagesTimeline.tsx:1483`).
- **orca** uses `@tanstack/react-virtual`, overscan 6 (`native-chat/viewport/virtualization/use-native-chat-transcript-window.ts:23`). It estimates row heights, measures mounted rows, and keeps offsets through content-only growth (lines 83–101).
- **Verdict:** skip both for v1. A journal capped at a few hundred rows needs no virtualisation. Keep the principle: estimate, then measure what is mounted.

### 5.6 Other transcript bits

- **Timestamps.** Show a short time per row; full time on hover (`MessagesTimeline.tsx:317`, `TimelineRowTimestamp`). **Adapt.**
- **Turn minimap** (`timelineMinimapItems.ts`). **Skip for v1.**

---

## 6. Diff view

Neither repo has a per-change accept or reject. Both show code diffs. We show artifacts, checks and journal entries. So most of this section is skip.

- **t3code `DiffPanel.tsx`** (1829 lines). Controls include "Split diff view" / "Stacked diff view" (line 1477), "Hide test files" (1533), "Hide whitespace changes", "Collapse diff" / "Expand diff". Engine: `@pierre/diffs` with a worker pool. **Skip.** [R] [I] [D] [X].
- **t3code `components/diffs/diffFileTree.logic.ts`** (135 lines). Builds a folder tree from a file list, with collapse-all and expand-all. Imports `lib/diffRendering.ts` (416 lines) and `pierre-icons.ts` (287). **Adapt** only if we list artifacts in folders.
- **orca `editor/diff/monaco-view/DiffViewer.tsx`** (438 lines). Monaco. **Skip.** [X]
- **orca `editor/diff/sections/diff-line-stats.ts`** (66 lines). `computeLineStats` (line 2) returns added and removed counts. It matches lines as multisets, so it is order-blind: a moved line counts as unchanged. Returns null over 500,000 characters. **Adopt** for "+3 −1" badges. Drag: none.
- **orca `diff-comments/diff-comment-line-range.ts`** (92 lines). `orderLineRange` (line 27), `clampFocusLineToCommentable` (52), `getSelectionAnchorFocus` (84). Pure. **Adopt** if we comment on line ranges.
- **orca `diff-comments/diff-comment-popover-position.ts`** (80 lines). `getDiffCommentPopoverTop` (line 12). Pure. **Adopt** with the file above.
- **orca `diff-comment-range-drag.ts`** (351 lines). Drag-to-select. **Skip** for now.

### 6.1 What exists instead of accept or reject

- **t3code approvals.** Pending agent requests show "Waiting for approval". Sending a new message while approvals wait declines them and carries your message as feedback (`apps/web/src/components/chat/useComposerSend.ts:1206–1212`). The nearest thing to our send-back.
- **orca review notes.** "Send Review Notes to Agent" (keybinding `sourceControl.sendReviewNotes`, no default key). A menu in `editor/review/DiffNotesSendMenu.tsx`.
- **orca stale saves.** Copy for a file changed on disk: "Keep My Edits" and "Overwrite" (section 9.2).
- **Verdict:** design our gate ourselves. Borrow the wording: "send back with a note", and two clear verbs for conflicts.

---

## 7. Status and progress

### 7.1 State words

- **t3code thread status** (`components/sidebar/SidebarThreadRow.tsx:789–850`): Working, Goal, Waiting, Approval, Input, Request, Limited, Failed, Woke, Done.
- **t3code announcer** (`packages/client-runtime/src/state/threadActivityAnnouncement.ts:9–16`): Working, "Needs approval", "Needs your input", Done, Failed, Stopped.
- **orca agent labels** (`components/agent/status/AgentStateDot.tsx:25–47`): Working, Monitoring background tasks, Blocked, Waiting for input, Interrupted, Failed, Done, Idle, No recent update, Couldn't confirm, Needs attention.
- **Copy from orca:** "Couldn't confirm" is not "Failed". A check we could not run is different from a check that ran and failed. A finished item nobody has seen gets an "· Unread" suffix (`workspace.activity.unreadLabel`).
- **Verdict:** adopt the words.

### 7.2 Window title says the status (t3code)

- `Sidebar.logic.ts:1007–1026`, `resolveWindowTitleThreadStatus`. Returns "Needs approval", "Needs your input" or "Working" for the open thread. A thread waiting on a child still counts as working.
- **For us:** the Tauri title can read "Ilvuno · 1 needs you". **Adapt.** About 20 lines.

### 7.3 Working time, progress, spinner, shimmer

- **Working time.** `formatWorkingDurationLabel` (`models.ts:419–430`). **Adopt.**
- **Progress ring** (`orca components/agent/status/AgentProgressRing.tsx`, 70 lines). SVG with `pathLength=100`, fills clockwise from 12 o'clock, `role="progressbar"`. Minimum visible arc **8%** (line 7): a 1% arc reads as "no progress". Shows only when the program reports 0–100. **Adapt** as a 25-line SVG function. [R] in the original.
- **Spinner** (`AgentWorkingSpinner.tsx`). CSS ring, rotated on the compositor. Under reduced motion it shows a static ring. **Adapt** the CSS.
- **Shimmer** (`orca/src/renderer/src/assets/text-shimmer.css`, 117 lines). Tailwind v4 `@utility` syntax. MIT notice (L4). **Adapt** as about 20 lines of plain keyframes with `background-clip: text`.

### 7.4 Failure display

- t3code: red "Failed"; "Limited" is separate for usage limits. orca: "Interrupted" is separate from "Failed".
- **Verdict:** adopt the split. A stop you asked for is not a failure.

---

## 8. Connection state

### 8.1 Connection dot (t3code)

- `apps/web/src/components/ConnectionStatusDot.tsx` (75 lines). Colour by phase (`connectionPhaseDotClassName`, lines 6–18): connected → success; connecting or reconnecting → warning, with a slow ping (line 20); error → destructive; other → muted at 40%.
- **Verdict:** adapt as a vanilla function, about 30 lines. Use the native `title` instead of its tooltip [D].

### 8.2 Connection words (t3code `apps/web/src/i18n/translations/en.json`)

- "Offline" (519) · "Connecting..." (520) · "Reconnecting..." (523) · "Connected" (524) · "Connection failed" (527)
- "Failed to connect. Reconnecting... Reason: {error}" (522)
- "Reconnected on t3@{version}." (2490)
- **Verdict:** adopt. Keep the reason in the text.

### 8.3 Cause-specific banners (orca)

- `terminal-pane/pty-transport/remote-runtime/recovery/TerminalRemoteRuntimeReconnectBanner.tsx`. Each cause has its own card and button: server replaced ("This host runs a different Orca server"); pairing refused ("This host no longer accepts this computer" / "Nothing on the host was stopped. Reconnect to try again."); sign-in needed.
- Words: "Can't reach Orca server" and "Check that Orca is running on this server and that your network connection is working, then try again." (`en.json`, `auto.store.slices.runtime.status.*`).
- **Verdict:** adapt. Say what was not touched. For us: "Local API is down. Runs keep going. The board will catch up."

### 8.4 Backoff (t3code)

- `apps/web/src/connection/platform.ts:627`: retry with backoff, because a backend can restart on the same port. **Adapt** for SSE: short delay, grow it, cap it.

---

## 9. Empty and error copy

These are the actual strings.

### 9.1 Empty states

| Where | Copy |
|---|---|
| t3code `components/NoProjectsHero.tsx:38` | "Add a project to start your first thread." |
| orca `worktreeJumpPalette.filter` | "No results match the active filter" / "Clear the filter above, or widen it to more hosts and projects." |
| orca task page, nothing in scope | "No issues in this workspace scope. Try searching or adjusting teams." |
| orca task page, filters hide all | "No issues match the selected filters. Clear a filter or try different criteria." |
| orca PR comments | "No comments yet." / "No bot comments." / "No human comments." |

The task-page pair is the best pattern: "nothing here" and "your filters hide everything" get different words and different fixes.

### 9.2 Errors and conflicts

| Where | Copy |
|---|---|
| orca `dashboardPopout.recoverableError` | "Orca dashboard hit an error." / "The dashboard could not finish rendering. Retry to remount it, or reopen it." |
| orca `folderWorkspacePathStatus` | "Folder not found" / "Orca cannot find {{path}}. Remove and re-import this folder workspace." |
| orca `sidebar.createProject.errors` | "Name cannot be empty" · "{{name}}" already exists at this location and is not empty. |
| orca stale-file banner | "This file changed on disk while you have unsaved edits. Overwrite replaces the newer disk content." Buttons: "Keep My Edits" · "Overwrite". |
| orca stale agent row | "Agent's pane is no longer available." |
| t3code `components/chat/TimelinePendingApprovalRow.tsx:110` | "Provider process is gone. Interrupt or restart the run to respond." |
| t3code `en.json:3124` | "Could not reach this environment. Try again." |

Rules from these: say what happened, then what to do. Name the thing that failed (path, host, file). For a conflict, offer two verbs, not "OK" and "Cancel".

### 9.3 Error block shape (orca)

- `new-workspace/composer/NewWorkspaceComposerFooter.tsx:40–50`. `role="alert"`, then a title, a message and an optional help line.

---

## 10. Keyboard and command palette

### 10.1 Keybinding model (t3code)

- `packages/shared/src/keybindings.ts:58`, `DEFAULT_KEYBINDINGS`: a list of `{ key, command, when }`. The VS Code model.
- `when` names: `terminalFocus`, `composerFocus`, `previewFocus`, `turnRunning`, `rightPanelOpen`.
- `mergeWithDefaultKeybindings` (line 395): a user rebind drops the default rules for that command. Other defaults stay.
- **Verdict:** adapt. Copy the shape and the merge rule, about 40 lines. Skip the parser.

### 10.2 Bindings worth copying (Mod = Ctrl or Cmd)

| Key | Action | Source |
|---|---|---|
| Mod+K | Command palette | t3 `mod+k`; orca `worktree.palette` |
| Mod+B | Toggle sidebar | t3 `mod+b`; orca `sidebar.left.toggle` |
| Mod+1 … Mod+9 | Select item 1–9 | orca `workspace.selectByIndex` |
| Mod+Shift+ArrowUp / Down | Previous / next item | orca `worktree.navigateUp` / `navigateDown` |
| Mod+Enter | Send the alternate way (queue or steer, section 12.2) while a turn runs | t3 `composer.sendAlternate` (`composerFocus && turnRunning`) |
| Mod+Shift+Enter | Steer the queued message | t3 `thread.steerQueuedMessage` |
| Alt+ArrowUp | Edit the queued message | t3 `thread.editQueuedMessage` |
| Mod+F | Find in the current view | orca `chat.find` |
| F7 / Shift+F7 | Next / previous change | orca `editor.nextChange` / `previousChange` |
| Mod+Shift+F | Search the project | t3 `projectSearch.toggle` |

Suggestion, not from either app: F7 could jump to the next Needs You item.

### 10.3 Focus across a virtual list (orca)

- `board/agent-kanban-lane-tab-bridge.ts` (39 lines). A virtualised list unmounts off-screen cards, so Tab would skip them. The bridge finds the logical neighbour and focuses it with `preventScroll`. **Adapt** only if we virtualise.

### 10.4 Palette

- **t3code** `apps/web/src/components/CommandPalette.tsx` (3209 lines). Groups: Projects, Change theme, Environments, Sources. Examples: "New thread in {project}" (line 1362), "Find in current thread" (1415), "Copy prompt link" (1472), "Restart agent session" (1560). **Skip the component.** Copy the command-list idea.
- **orca `shared/palette-match/`** (about 2,100 lines in total). Take three small parts:
  - `typo-distance.ts` (40 lines): one-edit typo check, for words of 4+ characters (line 1).
  - `match-quality.ts`: ladder, best first: field-exact, word-exact, field-prefix, word-prefix, boundary-substring, literal-substring, compact, typo.
  - `normalized-text.ts` (101 lines): `normalizePaletteText` (line 29), no imports.
- **Verdict:** adapt the three (about 150 lines). Skip the rest.

---

## 11. Design tokens and contrast

### 11.1 Our proto (`projects/ilvuno-second/proto/4-mix.html:3–4`)

Paper `#f6f4ef` · paper-2 `#efece4` · ink `#16150f` · ink-2 `#56524a` · ink-3 `#8a857a` · rule `#d8d3c7` · accent `#9e3b22` · green `#2f5d3a`.

### 11.2 Contrast (computed for this report)

| Colour | On paper | On paper-2 | On card `#fbfaf6` |
|---|---|---|---|
| ink | 16.64 | 15.49 | — |
| ink-2 | 7.07 | 6.58 | — |
| **ink-3 (now)** | **3.34** | **3.11** | **3.52** |
| accent | 6.16 | 5.73 | — |
| green | 6.95 | 6.47 | — |

**Fail.** ink-3 is used for most small mono labels, round numbers, table headers and the footer. WCAG AA needs 4.5:1.

**Fix.** Set `--ink-3` to `#6e6a5e`. That gives 4.92 on paper, 4.58 on paper-2, 5.17 on the card. Re-check after any colour change.

### 11.3 t3code stock palette (`packages/shared/src/themePalettes.ts:110` light, `:174` dark)

| Role | Light | Dark |
|---|---|---|
| canvas | `#fcfcfc` | `#0a0a0a` |
| surface | `#ffffff` | `#111111` |
| text / muted | `#27272a` / `#71717b` | `#f5f5f5` / `#818181` |
| border / input | `#e4e4e7` / `#d4d4d8` | `#191919` / `#1e1e1e` |
| focus | `#1b4ed8` | `#346bf1` |
| error (text, surface) | `#c10007`, `#fcebec` | `#ff6467`, `#301214` |
| warning (text, surface) | `#bb4d00`, `#fcf4e8` | `#ffb900`, `#312108` |

Copy the naming: each status colour has a base, a text colour and a banner surface. Not the values.

Contrast helper: `apps/web/src/themePalette.ts:975–1003` (`readableThemeForeground`, `readableThemeText`). Picks black or white text by ratio. **Adapt** the check. Do not copy the file.

### 11.4 orca stock palette (`renderer/src/assets/main.css`, `:root` line 213, `.dark` line 358)

| Role | Light | Dark |
|---|---|---|
| background / foreground | `#fff` / `#0a0a0a` | `#0a0a0a` / `#f0f0f0` |
| muted-foreground | `#696969` | `#acacac` |
| border | `#e5e5e5` | `rgb(255 255 255 / 0.07)` |
| ring | `#a1a1a1` | `#737373` |
| destructive | `#e40014` | `#ff6568` |
| agent-working / agent-question | yellow-500 / orange-600 | yellow-500 / orange-500 |

Radius `0.625rem` (line 224). The working-calm and question-accent pair is a good idea for us.

### 11.5 Type scale

- **t3code** (`index.css:184–191`): 2xs 11px, 3xs 10px, 4xs 8px, 5xs 7px.
- **orca** (`main.css:48–57`): 2xs 11px, 3xs 10px, settings detail 12.5px, settings heading 15px, chat code 12px.
- **Ours:** body 15.5px, h1 27px, labels 11px, table headers 9.5px. **Advice:** do not go below 10px for meaningful text. Raise the 9.5px table headers to 10px.

### 11.6 Fonts and dark mode

- t3code uses Inter Variable. orca bundles Geist (OFL). **For v1, keep the system stack.**
- orca switches on `.dark`. t3code switches on `html[data-theme-id]`. Our proto is light only. Decide dark mode after v1.

---

## 12. Composer and task creation

### 12.1 orca new-workspace composer

- Folder `new-workspace/composer/`. Sections: name, project, agent, advanced.
- **Default agent** from settings (`settings.defaultTuiAgent`, `NewWorkspaceComposerModal.tsx:184–188`).
- **Footer** (`NewWorkspaceComposerFooter.tsx`): error block with `role="alert"` (line 40); "Create more" switch (line 71), which keeps the dialog open; primary "Create" button whose `aria-label` is the full action (line 81), with a keyboard hint.
- **Verdict:** adapt the footer and error block. Skip the modal (416 lines).

### 12.2 t3code composer

- `components/chat/ChatComposer.tsx` (4184 lines), on `ComposerPromptEditorTiptap.tsx` (2100 lines). **Skip** the editor. [R] [D] [X].
- Placeholder: "Ask anything..." (`ChatComposer.tsx:3768`).
- **While a turn runs** (`components/chat/ComposerPrimaryActions.tsx:430`): "Click to queue, {modifier}-click to steer". Queue copy: "{count} queued", "Queued message {position} of {total}, not sent".
- **Verdict:** adapt the queue-or-steer idea for "send a note to a running task".

### 12.3 Validation

- orca shows create errors in one block (section 9.3). Name checks use plain messages.
- I did not trace every validation path in t3. It also uses Effect `Schema` for some parsing.
- **Advice:** validate the task prompt before the local API call. Show "Prompt is empty" under the field. Do not disable the button silently.

---

## 13. Pieces to lift

| # | File (clone) | Lines | What it gives us | Deps | Verdict |
|---|---|---|---|---|---|
| 1 | `t3code/apps/web/src/lib/preserveReadingPosition.ts` | 115 | Keeps reading position through re-layout | `observeResize` | adopt (after edit) |
| 2 | `t3code/apps/web/src/lib/observeResize.ts` | 96 | ResizeObserver with batching | `react-dom` (line 1) | adopt (drop `flushSync`) |
| 3 | `t3code/packages/client-runtime/src/state/models.ts:419–430` | 12 | `formatWorkingDurationLabel` | none | adopt |
| 4 | `orca/src/shared/agent-status/presentation/agent-row-decay-state.ts` | 67 | Stale → unverifiable/idle; "No update in Nm" | `freshness.ts` | adopt |
| 5 | `orca/src/shared/agent-status/contracts/agent-status-freshness.ts` | 79 | 30-minute rule | types only | adopt |
| 6 | `orca/…/dashboard/snapshot/dashboard-card-bucket.ts` | 19 | State → lane | types only | adopt (our states) |
| 7 | `orca/…/editor/diff/sections/diff-line-stats.ts` | 66 | `computeLineStats` (order-blind) | none | adopt |
| 8 | `orca/…/diff-comments/diff-comment-line-range.ts` | 92 | Line range helpers | none | adopt |
| 9 | `orca/src/shared/palette-match/typo-distance.ts` | 40 | One-edit typo check | none | adapt |
| 10 | `orca/src/shared/palette-match/normalized-text.ts` | 101 | Text folding | none | adapt |
| 11 | `t3code/packages/shared/src/chatList.ts` | ~40 | Pin a chosen item near top | none seen | adapt |
| 12 | `t3code/apps/web/src/components/Sidebar.logic.ts:779–1032` | ~250 | Unseen, status, folder-dot rules | many | adapt (rewrite) |
| 13 | `orca/src/renderer/src/attention/agent-attention-acknowledgement.ts` | 109 | `isAgentTurnUnacknowledged` (line 43) | one sibling file | adapt |
| 14 | `orca/…/agent/status/AgentProgressRing.tsx` | 70 | Progress ring, 8% minimum | [R] | adapt as SVG |
| 15 | `orca/src/renderer/src/assets/text-shimmer.css` | 117 | Shimmer text (Tailwind v4) | Tailwind | adapt (L4) |
| 16 | `orca/…/board/agent-kanban-lane-tab-bridge.ts` | 39 | Tab across unmounted rows | DOM | adapt later |
| 17 | `t3code/apps/web/src/components/ConnectionStatusDot.tsx` | 75 | Phase → colour and ping | [D] tooltip | adapt |
| 18 | `t3code/apps/web/src/themePalette.ts:975–1003` | ~30 | Readable text by contrast | none | adapt |
| 19 | `t3code/packages/shared/src/keybindings.ts:58, 395` | ~60 | Rule table and merge | shared package | adapt |

Copy strings, not files: the state words (7.1), the empty and error copy (9), the `dashboardPopout.*` keys in `orca/src/renderer/src/i18n/locales/en.json`, and the connection words (8.2).

---

## 14. What I did not find

- **A whole-board empty state.** Only "None" for an empty lane. No "No runs yet" in either repo.
- **A gate with accept and send-back.** Neither repo has one. t3's approval decline-on-send is the nearest.
- **Per-change accept or reject in a diff.** Neither repo has it.
- **An SSE reconnect that matches our stream.** t3 uses an RPC layer with backoff (8.4).
- **npm licence checks** (L8). Not done here.

---

## 15. Top 5 for our window this week

First: fix `--ink-3` (section 11.2). Fifteen minutes. It fixes a real contrast failure.

1. **Bucket and staleness rules.** Four lanes. Done stays marked until opened. A silent live run says "No update in Nm" and moves to Idle.
   - Evidence: `orca/…/dashboard/snapshot/dashboard-card-bucket.ts:1–19`; `orca/src/shared/agent-status/presentation/dashboard-snapshot.ts:35–42`; `…/agent-row-decay-state.ts:1–67`; `…/dashboard/rows/dashboard-row-bucket.ts:14`.
   - Estimate: **1 hour.** One pure module of about 60 lines, plus one test per rule.

2. **Copy deck and two formatters.** One `copy.js` with the state words, the empty and error copy, and the connection words. Add the working-time and "ago" formatters.
   - Evidence: sections 7.1, 9; `t3code/packages/client-runtime/src/state/models.ts:419–430`; `orca/…/agent-row-decay-state.ts:49–60`.
   - Estimate: **1 hour.**

3. **Reading-position helper for the journal.** Port `preserveReadingPosition.ts` with `flushSync` removed. Use it on the journal and the round log.
   - Evidence: `t3code/apps/web/src/lib/preserveReadingPosition.ts:1–115`; `t3code/apps/web/src/lib/observeResize.ts:1, 44`.
   - Estimate: **2 hours.** Most of it is testing in the Tauri webview.

4. **Connection dot and cause-specific words.** A 30-line dot for connected, reconnecting, error and idle. Messages that say what was not touched. Backoff on reconnect.
   - Evidence: `t3code/apps/web/src/components/ConnectionStatusDot.tsx:1–75`; `t3code/apps/web/src/i18n/translations/en.json:519–527`; `orca/…/TerminalRemoteRuntimeReconnectBanner.tsx`; `t3code/apps/web/src/connection/platform.ts:627`.
   - Estimate: **1 hour.**

5. **Keymap table and command list.** t3's rule shape (`key`, `command`, `when`) with its merge rule. Start with the bindings in section 10.2. Build the palette as grouped commands. Skip fuzzy matching for v1.
   - Evidence: `t3code/packages/shared/src/keybindings.ts:58, 395`; `t3code/apps/web/src/components/CommandPalette.tsx`; `orca/src/shared/keybindings/definitions-core-1.ts`.
   - Estimate: **2 hours.**

**Honest answer on copying.** We should copy almost no code. Copy the behaviour, the strings, and the small pure files in section 13. The most useful part of either repo is orca's board logic, which is about 200 lines of rules, not UI.
