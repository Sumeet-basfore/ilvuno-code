#!/usr/bin/env python3
"""Render one full-window mock of the app from an exported run.

    python3 tools/render-app-mock.py fixtures/board-experiment mock

Dark workbench: runs on the left, the board in the centre, the run's facts on the right.
Same data as the prototypes; this is about the shell, not new content.
"""

from __future__ import annotations

import html
import json
import re
import sys
from pathlib import Path

CSS = """
:root{
  --bg:#131216; --pane:#19181d; --pane-2:#1e1d23; --line:#2a282f; --line-2:#37343d;
  --ink:#e9e6df; --ink-2:#b3ada2; --ink-3:#8d877c;
  --accent:#e06a43; --accent-dim:#3a241d; --green:#7fae7a; --green-dim:#22301f;
  --red:#e0705c; --red-dim:#37201c; --amber:#d8a657;
  --mono:"JetBrains Mono","Liberation Mono",ui-monospace,monospace;
  --sans:"Noto Sans","Liberation Sans",Inter,system-ui,sans-serif;
  --r:6px;
}
*{box-sizing:border-box}
html,body{height:100%}
body{margin:0;background:var(--bg);color:var(--ink);font-family:var(--sans);font-size:13.5px;
  line-height:1.55;-webkit-font-smoothing:antialiased;overflow:hidden}
.app{display:grid;grid-template-columns:268px minmax(0,1fr) 316px;height:100vh}

/* ---------------- left rail ---------------- */
.rail{background:var(--pane);border-right:1px solid var(--line);display:flex;flex-direction:column;min-height:0}
.rail .top{display:flex;align-items:center;gap:8px;padding:12px 12px 8px}
.brand{font-family:var(--mono);font-size:11.5px;letter-spacing:.16em;text-transform:uppercase;color:var(--ink-2)}
.brand b{color:var(--accent);font-weight:500}
.rail .tools{margin-left:auto;display:flex;gap:6px}
.iconbtn{width:24px;height:24px;display:grid;place-items:center;border:1px solid transparent;border-radius:4px;
  color:var(--ink-3);background:none;cursor:pointer;font-size:13px}
.iconbtn:hover{background:var(--pane-2);color:var(--ink);border-color:var(--line)}
.search{margin:0 12px 10px;display:flex;align-items:center;gap:7px;padding:6px 9px;background:var(--pane-2);
  border:1px solid var(--line);border-radius:var(--r);color:var(--ink-3);font-size:12.5px}
.search span{opacity:.7}
.newrun{margin:0 12px 12px;padding:7px 10px;border:1px solid var(--line-2);border-radius:var(--r);
  background:transparent;color:var(--ink-2);font-size:12.5px;display:flex;align-items:center;gap:8px;cursor:pointer}
.newrun:hover{border-color:var(--accent);color:var(--ink)}
.group{padding:8px 12px 4px;font-family:var(--mono);font-size:9.5px;letter-spacing:.13em;
  text-transform:uppercase;color:var(--ink-3)}
.run-row{display:block;padding:9px 12px 10px;border-left:2px solid transparent;color:inherit;text-decoration:none}
.run-row:hover{background:var(--pane-2)}
.run-row.on{background:var(--pane-2);border-left-color:var(--accent)}
.run-row .t{font-size:13px;line-height:1.35;font-weight:600;display:-webkit-box;-webkit-line-clamp:2;
  -webkit-box-orient:vertical;overflow:hidden}
.run-row .m{display:flex;align-items:center;gap:7px;margin-top:6px;font-family:var(--mono);font-size:10.5px;
  color:var(--ink-3)}
.dot{width:6px;height:6px;border-radius:50%;background:var(--ink-3);flex:none}
.dot.ok{background:var(--green)} .dot.bad{background:var(--red)} .dot.busy{background:var(--amber)}
.rail .foot{margin-top:auto;border-top:1px solid var(--line);padding:9px 12px;display:flex;gap:10px;
  font-family:var(--mono);font-size:10px;color:var(--ink-3)}

/* ---------------- centre ---------------- */
.main{display:flex;flex-direction:column;min-height:0;background:var(--bg)}
.crumbs{display:flex;align-items:center;gap:8px;padding:11px 20px;border-bottom:1px solid var(--line);
  font-family:var(--mono);font-size:11px;color:var(--ink-3);flex:none}
.crumbs .sep{opacity:.5}
.crumbs .live{margin-left:auto;display:flex;align-items:center;gap:6px;color:var(--green)}
.scroll{overflow:auto;padding:18px 26px 8px;flex:1;min-height:0}
.doc{max-width:760px;margin:0 auto}
h1{font-size:21px;line-height:1.3;letter-spacing:-.012em;margin:4px 0 8px;font-weight:650}
.done{color:var(--ink-2);font-size:13px;margin:0 0 12px}
.chips{display:flex;flex-wrap:wrap;gap:7px;align-items:center;margin:0 0 18px}
.chip{font-family:var(--mono);font-size:9.5px;letter-spacing:.1em;text-transform:uppercase;padding:2px 6px;
  border-radius:3px;border:1px solid var(--line-2);color:var(--ink-2)}
.chip.ok{color:var(--green);border-color:#2f4a2b;background:var(--green-dim)}
.chip.bad{color:var(--red);border-color:#4a2b25;background:var(--red-dim)}
.chip.num{text-transform:none;letter-spacing:.02em;color:var(--ink-3)}
.gate{border:1px solid var(--accent);background:var(--accent-dim);border-radius:var(--r);padding:12px 14px;margin:0 0 20px}
.gate .k{font-family:var(--mono);font-size:9.5px;letter-spacing:.13em;text-transform:uppercase;color:var(--accent)}
.gate p{margin:5px 0 9px;color:var(--ink-2);font-size:12.5px}
.gate textarea{width:100%;height:46px;resize:none;background:#181418;color:var(--ink);border:1px solid #4a332b;
  border-radius:5px;padding:7px 9px;font-family:inherit;font-size:12.5px;margin-bottom:9px}
.gate textarea::placeholder{color:#7d6d64}
.btns{display:flex;gap:8px}
button{font-family:var(--sans);font-size:12.5px;padding:6px 12px;border-radius:5px;border:1px solid var(--line-2);
  background:transparent;color:var(--ink-2);cursor:pointer}
button:hover{border-color:var(--ink-3);color:var(--ink)}
button.primary{background:var(--accent);border-color:var(--accent);color:#1a0f0b;font-weight:650}
button.primary:hover{filter:brightness(1.08)}
h2{font-family:var(--mono);font-size:9.5px;letter-spacing:.13em;text-transform:uppercase;color:var(--ink-3);
  font-weight:400;margin:26px 0 10px;display:flex;justify-content:space-between;align-items:baseline}
h2 span{letter-spacing:.02em;text-transform:none;font-size:10px}
.round{display:grid;grid-template-columns:64px minmax(0,1fr);gap:12px;padding:9px 0;border-top:1px solid var(--line)}
.round .n{font-family:var(--mono);font-size:10.5px;color:var(--ink-3);padding-top:2px}
.round .h{display:flex;gap:8px;align-items:center;font-size:12.5px;font-weight:600}
.round p{margin:3px 0 0;color:var(--ink-2);font-size:12.5px}
.card{border:1px solid var(--line);background:var(--pane);border-radius:var(--r);padding:11px 13px;margin:0 0 8px}
.card .h{display:flex;align-items:center;gap:8px;margin-bottom:5px}
.card .t{font-size:12.5px;font-weight:650}
.card .stat{margin-left:auto}
.card p{margin:0 0 6px;color:var(--ink-2);font-size:12.5px;line-height:1.5}
.card .meta{font-family:var(--mono);font-size:10px;color:var(--ink-3)}
details{margin-top:5px}
summary{cursor:pointer;font-family:var(--mono);font-size:10px;color:var(--ink-3)}
details pre{background:#141317;border:1px solid var(--line);border-radius:4px;padding:9px 10px;margin:6px 0 0;
  font-size:11.5px;line-height:1.5;max-height:180px;overflow:auto;white-space:pre-wrap}
.composer{flex:none;padding:10px 26px 16px;background:linear-gradient(180deg,rgba(19,18,22,0),var(--bg) 22%)}
.composer .box{max-width:760px;margin:0 auto;border:1px solid var(--line-2);background:var(--pane);
  border-radius:8px;padding:9px 10px 8px}
.composer .row1{display:flex;align-items:center;gap:8px;margin-bottom:7px}
.pill{display:inline-flex;align-items:center;gap:5px;font-family:var(--mono);font-size:10px;color:var(--ink-3);
  border:1px solid var(--line);border-radius:999px;padding:2px 7px}
.input{display:flex;align-items:center;gap:9px}
.input .ph{flex:1;color:var(--ink-3);font-size:13px}
.input .send{width:26px;height:26px;border-radius:50%;border:0;background:var(--accent);color:#1a0f0b;
  display:grid;place-items:center;font-size:13px;font-weight:700}

/* ---------------- right rail ---------------- */
.facts{background:var(--pane);border-left:1px solid var(--line);overflow:auto;padding:12px 14px 24px}
.facts .sec{border-bottom:1px solid var(--line);padding:10px 0 12px}
.facts .sec:last-child{border-bottom:0}
.facts .k{display:flex;align-items:center;gap:7px;font-family:var(--mono);font-size:9.5px;letter-spacing:.12em;
  text-transform:uppercase;color:var(--ink-3);margin-bottom:7px}
.facts .v{font-size:12.5px;color:var(--ink-2);word-break:break-all}
.facts .row{display:flex;justify-content:space-between;gap:10px;padding:4px 0;font-size:12px}
.facts .row .l{color:var(--ink-2)}
.facts .row .r{font-family:var(--mono);font-size:10.5px;color:var(--ink-3)}
.chk{display:flex;gap:7px;align-items:baseline;padding:3px 0;font-size:12px}
.chk .m{font-family:var(--mono);font-size:9.5px;width:44px;color:var(--ink-3)}
.chk .d{color:var(--ink-2);flex:1}
.bar{height:5px;border-radius:3px;background:var(--pane-2);overflow:hidden;margin:6px 0 4px}
.bar i{display:block;height:100%;background:var(--accent)}
.thread{display:flex;align-items:center;gap:8px;padding:6px 0;font-size:12px}
.thread .n{font-weight:600}
.thread .a{margin-left:auto;font-family:var(--mono);font-size:10px;color:var(--ink-3)}
.safety{font-family:var(--mono);font-size:9px;color:var(--amber);border:1px solid #4a3a22;background:#2a2115;
  border-radius:3px;padding:1px 4px}
.path{font-family:var(--mono);font-size:10.5px;color:var(--ink-3);line-height:1.5}
.hint{padding:2px 12px 0;font-size:11.5px;color:var(--ink-3);line-height:1.45}
.proj-row{display:flex;align-items:center;gap:8px;padding:5px 0;font-size:12.5px}
.proj-row .pn{font-weight:600}
.proj-row .pc{margin-left:auto;font-family:var(--mono);font-size:10px;color:var(--ink-3)}
"""


def repo_root(run: dict) -> str:
    proj = run["run"].get("project") or ""
    return str(Path(proj).parent) if proj else ""


def shrink(text: str, root: str) -> str:
    if root:
        text = text.replace(root + "/", "")
    text = re.sub(r"/(?:[\w.\-]+/)*runs/([0-9a-f\-]{36})/", r"runs/\1/", text)
    return re.sub(r"/(?:[\w.\-]+/)+artifacts/", "artifacts/", text)


def short_title(goal: str) -> str:
    first = goal.split(". ")[0].strip().rstrip(".")
    return first if len(first) <= 96 else first[:93].rsplit(" ", 1)[0] + "…"


def sentences(text: str, max_chars: int) -> str:
    text = re.sub(r"\s+", " ", text or "").strip()
    return text if len(text) <= max_chars else text[: max_chars - 1].rsplit(" ", 1)[0] + "…"


def chip(text: str, cls: str = "") -> str:
    return f'<span class="chip {cls}">{html.escape(text)}</span>'


def render(run: dict, force_reviewed: bool = False) -> str:
    r, g = run["run"], run["goal"]
    threads, tasks, rounds, ledger = run["threads"], run["tasks"], run["rounds"], run["ledger"]
    artifacts = run["artifacts"]
    status = "reviewed" if force_reviewed else r["status"]
    ok = status in ("accepted", "reviewed")
    project = r.get("project") or ""
    project_dir = project.rsplit("/", 1)[-1] or project

    # --- left rail ---
    rows = ""
    for t in threads:
        rows += f"""<div class="thread"><span class="dot ok"></span><span class="n">{html.escape(t["name"])}</span>
<span class="a">{html.escape(t["agent"])}</span></div>"""

    # --- centre ---
    gate = ""
    if status == "reviewed":
        gate = f"""<div class="gate">
  <div class="k">Gate · waiting for you</div>
  <p>The run reached its checks. Accept it, or send it back with a note the agents will read.</p>
  <textarea placeholder="What should change? The agents read this note."></textarea>
  <div class="btns"><button class="primary">Accept</button><button>Send back</button></div>
</div>"""

    round_rows = ""
    for rd in rounds:
        checks = (rd.get("readiness") or {}).get("checks") or []
        failed = [c["name"] for c in checks if not c["passed"]]
        who = ", ".join(t["thread"] for t in tasks if t["round"] == rd["index"])
        verdict = rd.get("verdict") or "open"
        cls = "ok" if verdict == "pass" else "bad"
        why = "every check passed" if not failed else "failed: " + "; ".join(failed)
        round_rows += f"""<div class="round"><div class="n">round {rd["index"]}</div>
  <div><div class="h">{html.escape(who)} {chip(verdict, cls)}</div><p>{html.escape(why)}.</p></div></div>"""

    cards = ""
    for i, t in enumerate(tasks, start=1):
        msg = t.get("message") or ""
        cls = "bad" if t["status"] == "failed" else "ok"
        cards += f"""<div class="card">
  <div class="h"><span class="t">{i} · {html.escape(t["thread"])}</span>
    <span class="stat">{chip(t["status"], cls)}</span></div>
  <p>{html.escape(sentences(shrink(t["prompt"], repo_root(run)), 210))}</p>
  <div class="meta">round {t["round"]} · by {html.escape(t["created_by"])}</div>
  {f'<details><summary>what the agent said · {len(msg):,} chars</summary><pre>{html.escape(msg[:3000])}</pre></details>' if msg else ''}
</div>"""

    # --- right rail ---
    checks_html = ""
    for rd in rounds:
        for c in (rd.get("readiness") or {}).get("checks") or []:
            cls = "ok" if c["passed"] else "bad"
            checks_html += (f'<div class="chk"><span class="m">r{rd["index"]}</span>'
                            f'<span class="d">{html.escape(c["name"])}</span>{chip("pass" if c["passed"] else "fail", cls)}</div>')

    arts = ""
    for a in artifacts:
        name = a["path"].split("/artifacts/", 1)[-1] if "/artifacts/" in a["path"] else a["path"].rsplit("/", 1)[-1]
        arts += f'<div class="row"><span class="l">{html.escape(name)}</span><span class="r">{a.get("bytes") or 0:,}b</span></div>'

    pct = min(100, round(r["tokens_used"] / max(1, g["budget"]["tokens"]) * 100))

    return f"""<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><title>Ilvuno — {html.escape(short_title(g["text"]))}</title>
<style>{CSS}</style></head><body><div class="app">

<aside class="rail">
  <div class="top"><div class="brand"><b>◆</b> Ilvuno</div>
    <div class="tools"><button class="iconbtn">⌕</button><button class="iconbtn">≡</button><button class="iconbtn">+</button></div>
  </div>
  <div class="search"><span>⌕</span> Search runs</div>
  <button class="newrun"><span>+</span> New run</button>
  <div class="group">Today</div>
  <a class="run-row on" href="#">
    <div class="t">{html.escape(short_title(g["text"]))}</div>
    <div class="m"><span class="dot {'ok' if ok else 'bad'}"></span>{html.escape(status)}
      <span>·</span>{r["rounds_used"]} rounds<span>·</span>{r["tokens_used"]:,} tok</div>
  </a>
  <div class="hint">One run in this project. “New run” starts another.</div>
  <div class="group" style="margin-top:14px">Projects</div>
  <div style="padding:0 12px 8px">
    <div class="proj-row"><span class="dot ok"></span><span class="pn">{html.escape(project_dir)}</span>
      <span class="pc">1 run</span></div>
  </div>
  <div class="foot"><span>⌘K commands</span><span>·</span><span>⌘N new run</span></div>
</aside>

<main class="main">
  <div class="crumbs"><span>{html.escape(project_dir)}</span><span class="sep">/</span>
    <span>{html.escape(short_title(g["text"])[:42])}</span>
    <span class="live"><span class="dot ok"></span> live</span></div>

  <div class="scroll"><div class="doc">
    <h1>{html.escape(short_title(g["text"]))}</h1>
    <p class="done">{html.escape(g.get("done") or "")}</p>
    <div class="chips">
      {chip(status, "ok" if ok else "bad")}
      <span class="chip num">{r["rounds_used"]} rounds</span>
      <span class="chip num">{r["tokens_used"]:,} tokens</span>
      <span class="chip num">{len(threads)} threads</span>
      <span class="chip num">{len(tasks)} tasks</span>
    </div>
    {gate}
    <h2>What happened <span>{len(rounds)} rounds</span></h2>
    {round_rows}
    <h2>Tasks <span>{len(tasks)}</span></h2>
    {cards}
    <div style="height:8px"></div>
  </div></div>

  <div class="composer"><div class="box">
    <div class="row1"><span class="pill">orchestrator</span><span class="pill">round 2 · open</span></div>
    <div class="input"><span class="ph">Add a task, or ask the orchestrator what to do next…</span>
      <button class="send">↑</button></div>
  </div></div>
</main>

<aside class="facts">
  <div class="sec"><div class="k">Project</div>
    <div class="path">{html.escape(project)}</div>
    <div class="row"><span class="l">Open in editor</span><span class="r">⌘O</span></div>
  </div>
  <div class="sec"><div class="k">Checks</div>{checks_html}</div>
  <div class="sec"><div class="k">Budget</div>
    <div class="row"><span class="l">{r["tokens_used"]:,} of {g["budget"]["tokens"]:,} tokens</span><span class="r">{pct}%</span></div>
    <div class="bar"><i style="width:{pct}%"></i></div>
    <div class="row"><span class="l">Rounds</span><span class="r">{r["rounds_used"]} / {g["budget"]["rounds"]}</span></div>
  </div>
  <div class="sec"><div class="k">Threads</div>
    {rows}
  </div>
  <div class="sec"><div class="k">Artifacts</div>{arts or '<div class="v">none</div>'}</div>
  <div class="sec"><div class="k">Gate</div>
    <div class="row"><span class="l">Decision</span><span class="r">{html.escape(status)}</span></div>
    <div class="row"><span class="l">Checks run</span><span class="r">{sum(len((rd.get("readiness") or {}).get("checks") or []) for rd in rounds)}</span></div>
  </div>
</aside>
</div></body></html>"""


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    fixture, out = Path(argv[0]), Path(argv[1])
    run = json.loads((fixture / "run.json").read_text())
    out.mkdir(parents=True, exist_ok=True)
    path = out / "app.html"
    path.write_text(render(run))
    gate = out / "app-gate.html"
    gate.write_text(render(run, force_reviewed=True).replace(
        "<title>", "<title>[state demo] ", 1))
    print(f"wrote {path} and {gate}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
