> Paste everything between the two markers as the goal. Nothing else.
> **Do this after the dogfood run.** Packaging a thing nobody has used is polish for the wrong thing.

---8<--- PROMPT STARTS ---8<---

**Goal: someone else can install Ilvuno on Linux and start it.**

Read `/home/sumeet/personal-repo/projects/ilvuno-second/context/prompts/README.md` and follow the house
rules in it. Roadmap stage 5.

## Today

There is no installer. `tools/window.sh` builds and runs from source with a Rust toolchain that has to be
pinned by hand. A person who is not a developer cannot start it.

## What to build

**1 · A build that produces something installable.**
- `tauri build` with bundling enabled for **deb** and **AppImage**. The window must be self-contained: the
  UI is embedded, so the binary plus its webkit dependency is the whole app.
- The app needs to find and start the runner. **Decide how, and say why**: options are (a) the window starts
  `ilvunod` itself if nothing answers on 4737, (b) the package installs a systemd user unit, (c) the window
  shows the offline banner until the runner is started by hand. Pick the smallest that works, and write the
  choice into `context/decisions.md`.

**2 · A first-run guide**, `docs/install.md`, plain English:
- what Ilvuno is, in three sentences;
- what to install first (one agent CLI, logged in);
- how to install Ilvuno, and how to start it;
- what to do when the window says the runner is not running;
- where its data lives (`ILVUNO_HOME`, default `~/.local/share/ilvuno`), and how to delete it.

**3 · A version that can be read.**
- `ilvuno --version` and the window's About tab must print the same version, from `Cargo.toml`.

**4 · Nothing that phones home.** No update check, no telemetry, no network call except 127.0.0.1.

## Where

- `crates/desktop/tauri.conf.json` — `bundle.active`, `targets`, icons, identifier.
- `crates/desktop/icons/` — an icon set (a simple gem mark is enough; do not design a brand).
- `docs/install.md`, and a line in `README.md` pointing at it.
- `crates/desktop/src/main.rs` if you choose to start the runner from the window.

## Rules for this goal

- No new dependency for packaging beyond what Tauri bundles.
- The binary must run from a clean machine: no `cargo run`, no `PATH` tricks, no `RUSTUP_TOOLCHAIN`.
- Do not add auto-update. It needs hosting, signing and a release process — all out of scope
  (`docs/prd.md` §6).
- Do not change how a run works. This goal is a wrapper, not a feature.

## Done when

- `RUSTUP_TOOLCHAIN=1.100.0 cargo build --release -p desktop` succeeds, and a `tauri build` produces a
  `.deb` and an `.AppImage` under `target/release/bundle/`;
- the AppImage starts on this machine, shows the window, and reads a real run from a real `ilvunod`;
- the `.deb` installs into a fresh `--root` (`dpkg -x` is enough to prove the layout), and its desktop entry
  starts the app;
- `docs/install.md` is accurate — every command in it has been run;
- `context/timeline.md` and `context/decisions.md` are updated.

## Stop and report instead, if

- a webkit version on this machine is outside Tauri's supported range — say so, with the version, and stop;
- bundling needs a signing key or an account;
- the AppImage runs but the window is blank — that is a real defect; report it rather than working around it.

---8<--- PROMPT ENDS ---8<---

## For you, before you paste

- **Cost:** build time; the AppImage build is slow the first time.
- **Check when it reports:** run the AppImage yourself from a folder that is not the repo. If it opens and
  lists runs, it is real.
- **Not in this goal:** auto-update, a package repository, Windows or macOS.
