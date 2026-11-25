# Ilvuno design system

This document turns the four supplied product screenshots into one design system for Ilvuno. It describes what each image shows, what Ilvuno should take from it, and how the rules apply to the current desktop app.

The direction is a quiet, dark workbench. Keep the thread conversation easy to read. Keep run state close. Make review and merge decisions obvious. Use color for actions and state, not decoration.

This system is implemented in `crates/desktop/ui/`. The token values below match the current window. The screenshots in `crates/desktop/shots/` show the rebuilt app against a saved run.

## Product fit

Ilvuno is a local desktop app for running coding agents in recorded rounds. A person chooses the agents, reads their work, and accepts or returns the result. The design system must support that work without turning the window into a general project manager.

Keep the existing product model:

- The left rail lists runs and threads.
- The centre shows the selected thread and its conversation.
- The right inspector shows shared run work and review details.
- The board stays a record of round notes. It is not a Kanban board.
- The human gate stays the final authority. The app never accepts work by itself.
- Tool activity stays available as a record. It does not become a terminal.
- Agents remain user choices. Do not assign fixed coding or review roles.

## What the references show

The screenshots are design references, not assets to copy. Their app names, logos, product text, and exact layouts stay with their owners.

### Raycast command search

Reference file: `05-raycast-command-search.png`. The reference pack names Mac O'Clock as the image source.

The image shows a dark command window over a light desktop. The window has rounded corners, a fine outline, and a soft shadow. A search field spans the top row. The placeholder says "Search for apps and commands…". "Ask AI" and a `Tab` key hint sit on the right. A divider separates the field from the results.

The results have two groups, "Suggestions" and "Commands". Each row places an app or command icon on the left, the name beside it, and a type on the right. One row is selected with a brighter gray background. A system command has a smaller source label beside its name. The visible examples include Telegram, Obsidian, Quit All Apps, Spotify, WeChat, and Create Reminder from Apple Reminders.

A bottom bar shows the selected action, its Enter key, an Actions label, and the `⌘ K` shortcut. A small utility mark sits at the bottom left.

Use this reference for Ilvuno's command palette. Keep the search field prominent. Group matches by runs, threads, and actions. Show an icon, a clear name, and a quiet type or context label. Keep one row selected at a time. Put the action and keyboard hint in a fixed footer. Linux uses `Ctrl K`, so do not copy the Mac shortcut. Ilvuno has no "Ask AI" command. Do not add one to the palette.

### Conductor agent workspace

Reference file: `04-conductor-agent-workspace.webp`. The source listed in the reference pack is Conductor's website.

The screenshot shows a dark desktop app inside a rounded window, with a black image background around it. The top edge has desktop window controls, a sidebar toggle, back and forward buttons, several open work tabs, a plus button, history, link sharing, collaborator avatars, and a split-view button. A green review strip on the right shows a pull request number, an external-link action, "Ready to merge", and a green Merge button.

The left sidebar has the Conductor mark, Home, Create, and Search. A Pinned section holds one workspace. A My workspaces section lists several workspaces with recent activity times. The bottom area has View all, Team members, a PRO mark, Help, and Settings.

The centre pane shows one agent conversation. The first user message contains a text request and an attached image. Agent messages follow as readable paragraphs. Tool calls appear as collapsed rows with counts. A later message shows another tool-call count, a concise completion message, a summary list, a commit identifier, a pull request link, a build status, and changed-file chips with added and removed line counts. A time and copy action sit above the file chips.

The composer sits at the bottom of the centre pane. It includes a recipient token, a text field, a model name, a reasoning setting, a context or reference control, an attachment control, and a send button. The right pane lists files and line changes. Tabs switch between All files, Changes, Checks, and Review. A lower panel has Run and Terminal tabs, an add button, a play control, and a development-start prompt.

Use this reference for Ilvuno's thread-first workspace, readable conversation, collapsible tool activity, file review, and a strong gate action. Keep the review controls close to the files they affect. Use a compact summary before raw tool details. Ilvuno does not ship an embedded terminal, so the Run and Terminal panel is not part of this system. The screenshot's shared team, model, and cloud controls also do not imply multi-user or cloud features for Ilvuno.

### Linear workspace

Reference file: `03-linear-workspace.png`. The local file uses a `.png` name, but its image data is AVIF. The reference pack lists Linear's conceptual model page as the source.

The screenshot shows a dark issue window over a pink-to-blue desktop background. Its header includes workspace identity, navigation controls, an "Active Issues" title with a count, a Filter action, a sort control, and an add button.

The left sidebar starts with a workspace mark and a New Issue control. It then lists Inbox, My Issues, Views, and Roadmap. A Favorites group contains saved views. A team group lists Design and X Dev. Invite people and Help & Support sit at the bottom.

The main area is a dense list grouped by workflow state. The visible groups are In Review, In Progress, and Todo, each with a count and a control to add an item. Each row combines a state icon, issue key, title, labels, an estimate or cycle marker, a date, and an assignee avatar. Labels use small colored pills. The selected or hovered row has a quiet gray fill. Dividers separate rows without wrapping every row in a card.

Use this reference for Ilvuno's board list, grouped status sections, compact rows, filters, counts, and quiet metadata. Keep round and task names readable before secondary details. Use small status marks and labels. Ilvuno's ledger is chronological evidence, so keep it grouped by round rather than changing it into a task-status board. The desktop gradient, issue IDs, estimates, due dates, team members, and workspace management do not apply to Ilvuno.

### GitButler worktrees

Reference file: `02-gitbutler-worktrees.png`. The reference pack lists GitButler's documentation as the source.

The screenshot shows a large light app window over a vivid blue, purple, pink, and orange desktop background. The app has rounded outer corners and desktop window controls. Its top bar shows recent activity, an upstream-commits count, a selected branch named "why", and an Actions button.

A narrow vertical toolbar sits at the far left. The main area has an Unassigned column and several Assigned columns. Each column has a count, list or tree controls, and a dotted drag area. File rows show a file-type mark, file name, folder path, and small added or changed indicators.

Some work items are expanded into branch cards. A card shows a branch name, integrated state, time, pull request badge, and a row of branch actions. One card has an upstream-commits section and a vertical commit line. A lower card has task steps on a colored timeline. Teal buttons start commits. Purple marks steps. Orange marks upstream changes. Green marks added or integrated work. The sidebar also shows account, support, keyboard, notification, and profile controls.

Use this reference for showing thread branches, changed files, integration state, commit history, and add or remove line counts. A file can be tied to the thread that produced it. Use a small lane or timeline when branch history matters. Do not turn Ilvuno's full board into the reference's draggable columns. The colorful desktop wallpaper is outside the app window. Keep Ilvuno's app canvas flat.

## Design rules

### Work has the strongest hierarchy

The selected conversation is the main reading area. Its prompt and agent response use the largest body text. The run title and status stay visible in the top bar. The inspector groups shared work into a small number of clear destinations.

When a gate opens, make Review the first inspector destination. Show the checks, thread changes, and decision controls together. Use one primary action. Keep the send-back action secondary.

During a run, keep the board and checks easy to reach. Move static run facts such as the full folder path and identifier into a compact summary or details view. Do not repeat the same data in the top bar, sidebar, and inspector.

### Use lists before cards

Use rows for threads, runs, files, checks, and board entries. Use a card only when a group needs its own actions or state, such as an open review decision or a merge conflict. Do not give every section its own rounded box.

Use separators to group related rows. Keep decorative separators subtle. Give controls and selected rows a clearer boundary.

### Keep the work record readable

Show user prompts as a lightly raised block. Show agent responses on the main canvas with a readable line length. Show a report as a concise summary with a clear status mark. Keep code and file paths in monospace. Keep ordinary prose in the sans font.

Collapse tool activity by default. The closed row states how many calls happened and how many were allowed or denied. The expanded view shows the tool name, target, permission result, and failure text. Keep the exact command available in a further detail view when the provider records it.

### Use color with meaning

Use the main action color for keyboard focus, selected navigation, and the primary action. Use semantic colors only for success, attention, failure, and information. Use text and an icon with each status color. Never rely on color alone.

Keep ledger kinds mostly neutral. Use a small source mark to distinguish user, agent, and runner entries. Do not assign a bright color to every kind of note.

## Color tokens

These values translate the charcoal surfaces, quiet selection, turquoise actions, and small semantic marks in the references. They are not sampled product colors.

| Token | Dark theme | Light theme | Use |
|---|---|---|---|
| `canvas` | `#151515` | `#F3F3F0` | Main reading area |
| `rail` | `#1B1B1B` | `#EBEBE6` | Navigation and inspector background |
| `surface` | `#202020` | `#FFFFFF` | Fields and raised content |
| `surface-hover` | `#292929` | `#F2F2EE` | Hovered rows and controls |
| `surface-selected` | `#303030` | `#E6E7E1` | Selected row or tab |
| `border` | `#363636` | `#D8D9D2` | Pane edges and row separators |
| `border-strong` | `#505050` | `#B8BAB2` | Emphasized separators and edges |
| `control-border` | `#777777` | `#777970` | Text fields and controls |
| `text-primary` | `#F2F2F0` | `#22231F` | Titles and body text |
| `text-secondary` | `#B8B8B2` | `#575850` | Supporting text |
| `text-muted` | `#999993` | `#66685F` | Counts and quiet metadata |
| `action` | `#3CC7B5` | `#08766D` | Primary action, selected edge, focus |
| `on-action` | `#09211D` | `#FFFFFF` | Text and icons on the main action color |
| `success` | `#70D28E` | `#267342` | Passed checks, accepted, merged |
| `attention` | `#E3B45C` | `#8C5A00` | Waiting, approval needed, approaching budget |
| `danger` | `#F08075` | `#B33B32` | Failed checks, blocked, destructive action |
| `information` | `#80A8F8` | `#356AD1` | Neutral informational state |
| `scrim` | `rgba(0, 0, 0, 0.58)` | `rgba(24, 22, 18, 0.28)` | Dialog and drawer backdrop |

The dark theme is the default. The light theme uses the same roles and state meanings. Do not use a different action color in each theme. The lowest tested text contrast is 4.59:1 for the light action color on the inspector background. The tested control borders are above 3:1 in both themes. Text must meet 4.5:1. Interactive boundaries must meet 3:1.

## Type and content

Use the bundled Inter font for the interface. Use the system monospace stack for code, paths, identifiers, counts, and shortcut keys. Do not use monospace for section titles or ordinary sentences.

| Style | Size | Weight | Use |
|---|---:|---:|---|
| Page title | 22 px | 600 | Board title or dialog title |
| Section title | 16 px | 600 | Thread, inspector, and settings headings |
| Body | 14 px | 400 | Agent messages, prompts, helper copy |
| Compact body | 13 px | 400 | Dense list rows and short summaries |
| Metadata | 12 px | 400 | Agent name, source, time, counts |
| Smallest label | 11 px | 400 or 500 | Key hint, state label, short identifier |

Use sentence case. Keep labels short. Do not use tracked all-caps section labels. Keep long text at about 60 to 72 characters per line. Use a line height between 1.45 and 1.65 for prose. Do not shrink body text to fit a pane. Let the pane scroll.

## Space, shape, and depth

Use a 4 px spacing unit. Common gaps are 4, 8, 12, 16, 24, and 32 px. Use 8 px between compact controls, 12 px inside rows, 16 px around sections, and 24 px around reading content.

| Shape token | Value | Use |
|---|---:|---|
| `radius-small` | 4 px | Chips and key hints |
| `radius-control` | 6 px | Buttons, inputs, and small menus |
| `radius-panel` | 8 px | Selected rows, message blocks, and review groups |
| `radius-dialog` | 12 px | Search palette, settings, and modal windows |

Keep the main application panes flat. Use a subtle shadow on floating menus and dialogs only. Use one-pixel borders for pane edges and row groups. A selected row uses a gray fill plus a 2 px action-colored edge. A hover row changes its fill without moving the layout.

Use a 16 px line icon inside a 32 px icon button. Give the button an accessible name. Use a 34 to 36 px height for standard buttons and fields. Use 20 to 22 px chips. Use 20 to 24 px provider marks or avatars. Do not use user avatars to suggest a team feature that Ilvuno does not have.

## Desktop layout

At 1440 px, use three columns. Keep the thread rail between 248 and 272 px. Keep the review inspector between 340 and 380 px. Let the conversation take the remaining width. Keep the reading column between 640 and 760 px when space permits.

The top bar holds the project and run name, one run status, round and token progress, and a gate action when a person needs to review. Do not place a second full set of run facts there.

The left rail holds the project and run selector, the gate shortcut when open, thread groups, a New thread action, and settings at the bottom. Group threads by Needs you, Working, and Done. Each row shows a status mark, thread name, selected agent, and current task summary.

The centre pane holds the selected thread header, conversation, and a composer fixed to the bottom. The composer uses the available width and grows to a limit before it scrolls. A closed composer is replaced by a reason and the next available action.

The right inspector groups run work into tabs or a short section list. Use Board, Changes, Checks, and Review as the first destinations. Show a count beside the label when the count helps. Select Review automatically when the gate opens. Keep the merge decision visible near the review summary.

At 1280 px and below, let the inspector close to give the conversation room. Open it as a drawer or overlay. At the 960 px minimum window width, keep the thread rail narrow and let the inspector cover part of the centre pane. Never compress three panes until each one becomes a narrow column.

## Components

### Command palette

Use a centered dialog with a width between 560 and 720 px and a maximum height of 70 percent of the window. Keep the search row at the top. Group results by Runs, Threads, and Actions. A row shows an icon, a result name, optional context, and a type aligned to the right. Highlight the selected row with a quiet fill.

Support `Ctrl K` to open, arrow keys to move, Enter to choose, and Escape to close. Keep the footer fixed while results scroll. Show the selected action and its key. Provide focus containment and return focus to the control that opened the palette.

### Navigation rows

Use a 36 to 44 px hit area. Show the state mark and name first. Put the agent and time at the end in muted text. Clamp the task summary to two lines. Use a filled background and left edge for the selected thread. Keep the selected row visible when the user opens its conversation.

### Conversation entries

Use 14 px text for prompts and agent responses. Keep user prompts in a quiet raised block. Keep agent responses on the main canvas. Put thread name, round, and report status in a small metadata row. Show report summaries with a clear success or failure edge. Keep file chips short and truncate long paths with an ellipsis.

### Inspector tabs and review

Use text tabs with a thin selection marker. Put counts beside tabs only when they help people choose. In Review, show the checks first, then changed threads and file summaries, then the decision note and actions. Use a prominent action for Accept and merge or Accept. Use a secondary action for Send back with note.

Show a conflict as a danger notice with affected file names and a plain explanation. Keep the exact patch in a disclosure so it is one step away from the summary.

### Board rows and worktree lanes

Group board rows by round. Each row shows the entry kind, author, short text, and related file. Use a thin vertical timeline when it clarifies order. Keep the default board as a list.

When the user reviews Git work, group changed files by thread or branch. Show file path, added and removed line counts, and status. Use a compact lane only when there are several parallel branches. Do not use draggable columns for the board ledger.

### Status marks and chips

Use a small icon or dot and a text label. Keep chips low in height and use them for short state or count labels. Do not wrap ordinary prose in chips. Use green only for verified success. Use amber for waiting or human attention. Use red for failure or blocked work. Use neutral gray for unknown. Use turquoise for selected or actionable controls.

### Buttons and fields

Use one primary button per region. Use an outline button for secondary actions. Keep destructive actions visually distinct and require confirmation when the action cannot be undone. Make the focus ring visible in both themes. Give every icon-only button an accessible name.

### Dialogs and notices

Use dialogs for focused tasks such as New run and Settings. Keep the title at the top and the primary action at the bottom. Keep file import available in New run. Make raw JSON an advanced option rather than the first field. Keep validation errors beside the setting or input they describe.

Use a full-width notice for offline, paused, budget, and blocked states. State what happened, what it means, and the next action. Keep the notice in the centre pane so it does not depend on the inspector being open.

## State and keyboard rules

| State | Mark | Copy pattern | Available action |
|---|---|---|---|
| Working | Neutral or blue dot | "Writer is working" | Stop |
| Waiting | Amber dot | "No task is waiting. Message a thread to continue." | Message thread |
| Review needed | Amber mark | "Review this round" | Open Review |
| Passed or merged | Green check | State the completed action | New run |
| Paused | Amber pause mark | Say who paused it and why | Resume |
| Budget exhausted | Amber or red budget mark | Show used and allowed tokens | Start a run with a larger budget |
| Failed or blocked | Red mark | Give the failure reason | Show details or resume if allowed |
| Unknown | Neutral mark | Say that the state is unknown | Retry or inspect the record |
| Offline | Amber warning mark | Say that the local runner is unreachable | Retry |

Use both a label and a mark for every state. Do not label a run as working when no agent is active. Do not offer Resume when the runner cannot resume that state.

Support the existing keyboard actions. Keep `Ctrl K` for the command palette, `Ctrl N` for New run, `Ctrl Enter` for Send, and `Escape` for closing a dialog or drawer. Keep focus inside an open dialog. Return focus when it closes. Respect reduced-motion settings. Keep transitions between 120 and 180 ms.

## Reference-to-Ilvuno mapping

| Ilvuno area | Reference pattern | Ilvuno rule |
|---|---|---|
| Command palette | Raycast search window | Group runs, threads, and actions. Show keyboard hints and a fixed action footer. |
| Thread workspace | Conductor conversation and review panes | Keep the conversation central. Put files, checks, and the gate in a review inspector. |
| Round board | Linear grouped issue list | Use compact, readable rows grouped by round. Keep filters and counts quiet. |
| Thread changes | GitButler worktree and commit lanes | Group files by thread or branch. Show changed lines and integration state. |
| Run states | Linear status marks and Conductor merge state | Use semantic color with a label. Keep success, attention, failure, and unknown distinct. |
| Small-screen desktop | Current Tauri minimum width | Hide or overlay the inspector before reducing the conversation width. |

## Exclusions

These screenshots contain useful patterns and product-specific features. Keep the patterns that fit Ilvuno. Do not add features just because a reference shows them.

- Do not add Raycast's Ask AI action, app launcher results, or Mac-only shortcuts.
- Do not add Conductor's cloud sharing, team roster, PR hosting, or embedded terminal.
- Do not add Linear's issue workflow, team management, estimates, due dates, or project wallpaper.
- Do not add GitButler's drag-and-drop board, force-push action, or colorful desktop wallpaper.
- Do not copy vendor logos, exact labels, or screenshot assets into Ilvuno.
- Do not add gradients to the application background.
- Do not use a colored card for every message, board entry, and status.
- Do not use color alone to communicate a state.

## Application review checklist

Checked against the desktop app on 2026-11-25. The screenshots use the real saved run
`64239160-0caa-456a-b3cf-3011a0f64bb1` from an isolated copy of its database. They were captured from the
built window at 1920×1080. The run is already merged, so it cannot show an open review gate.

- [x] The thread conversation has the strongest reading hierarchy.
- [x] The left rail shows which threads need attention.
- [x] The Review destination becomes selected when the gate opens. The transition is implemented; the saved run has no open gate to show it live.
- [ ] The person can see checks, changed files, and gate actions together. The layout is implemented, but the real saved run has a closed gate.
- [x] The board remains a chronological record grouped by round.
- [x] Tool activity is collapsed by default and readable when expanded.
- [x] New run does not require editing raw JSON as the normal path.
- [x] Dark and light themes use the same token roles and status meanings.
- [x] The measured text pairs meet 4.5:1 contrast, control borders meet 3:1, and the Board shortcut was checked with keyboard focus.
- [ ] The 960 px minimum width does not create three cramped columns. It needs a direct window check.
- [x] Screenshots show the current app code and current copy.

Screenshots: [thread](../crates/desktop/shots/2026-11-25-design-system-thread.png),
[board](../crates/desktop/shots/2026-11-25-design-system-board.png),
[palette](../crates/desktop/shots/2026-11-25-design-system-palette.png),
[New run](../crates/desktop/shots/2026-11-25-design-system-new-run.png),
[ACP option](../crates/desktop/shots/2026-11-25-design-system-acp-run.png),
[Changes](../crates/desktop/shots/2026-11-25-design-system-inspector-changes.png),
[Checks](../crates/desktop/shots/2026-11-25-design-system-inspector.png),
[closed Review](../crates/desktop/shots/2026-11-25-design-system-inspector-review-closed.png),
[Settings](../crates/desktop/shots/2026-11-25-design-system-settings.png), and
[light theme](../crates/desktop/shots/2026-11-25-design-system-light.png).
