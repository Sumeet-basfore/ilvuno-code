#!/usr/bin/env python3
"""Render three board layouts from an exported run.

    python3 tools/render-prototype.py fixtures/board-experiment proto

Same data, three arrangements, so a person can choose one. No JS beyond <details>, no build step.
"""

from __future__ import annotations

import html
import json
import re
import sys
from pathlib import Path

CSS = """
:root{
  --paper:#f6f4ef; --paper-2:#efece4; --ink:#16150f; --ink-2:#56524a; --ink-3:#8a857a;
  --rule:#d8d3c7; --accent:#9e3b22; --accent-soft:#f1e5dd; --green:#2f5d3a; --code:#efece4;
  --mono:"JetBrains Mono","Liberation Mono",ui-monospace,monospace;
  --sans:"Liberation Sans","Noto Sans",Inter,system-ui,sans-serif;
}
*{box-sizing:border-box}
body{margin:0;background:var(--paper);color:var(--ink);font-family:var(--sans);
  font-size:15.5px;line-height:1.6;-webkit-font-smoothing:antialiased}
.wrap{max-width:1100px;margin:0 auto;padding:40px 36px 96px}
a{color:var(--accent);text-decoration:none;border-bottom:1px solid var(--accent-soft)}
nav.variants{display:flex;gap:18px;font-family:var(--mono);font-size:11px;letter-spacing:.11em;
  text-transform:uppercase;color:var(--ink-3);margin-bottom:26px;flex-wrap:wrap}
nav.variants a{color:var(--ink-3);border-bottom:1px solid transparent;padding-bottom:2px}
nav.variants a.on{color:var(--accent);border-bottom-color:var(--accent)}
nav.variants a:hover{color:var(--ink)}
header.run{border-bottom:1.4px solid var(--ink);padding-bottom:16px;margin-bottom:26px}
header.run .proj{font-family:var(--mono);font-size:11px;letter-spacing:.06em;color:var(--ink-3);
  overflow-wrap:anywhere}
header.run h1{font-size:26px;line-height:1.18;letter-spacing:-.018em;margin:6px 0 10px;font-weight:700}
header.run p.done{color:var(--ink-2);font-size:14px;margin:0 0 14px;max-width:78ch}
.meta{display:flex;gap:22px;flex-wrap:wrap;font-family:var(--mono);font-size:11px;color:var(--ink-3)}
.meta b{color:var(--ink);font-weight:500}
.chip{display:inline-block;font-family:var(--mono);font-size:10px;letter-spacing:.1em;text-transform:uppercase;
  padding:2px 7px;border:1px solid var(--rule);color:var(--ink-2);margin-right:6px}
.chip.pass{color:var(--green);border-color:#b9c9b6;background:#eef2ea}
.chip.fail{color:var(--accent);border-color:#e0c3b7;background:var(--accent-soft)}
.chip.block{color:var(--accent);border-color:#e0c3b7}
.kind{font-family:var(--mono);font-size:9.5px;letter-spacing:.12em;text-transform:uppercase;color:var(--ink-3)}
h2.sec{font-size:12px;letter-spacing:.13em;text-transform:uppercase;font-family:var(--mono);
  color:var(--ink-3);font-weight:400;margin:44px 0 12px;border-top:1px solid var(--rule);padding-top:12px}
table{width:100%;border-collapse:collapse;font-size:13.5px}
th{text-align:left;font-family:var(--mono);font-size:10px;letter-spacing:.1em;text-transform:uppercase;
  color:var(--ink-3);font-weight:400;padding:0 10px 6px 0;border-bottom:1.1px solid var(--ink)}
td{padding:7px 10px 7px 0;border-bottom:1px solid var(--rule);vertical-align:top}
td.num{font-family:var(--mono);font-size:11.5px;color:var(--ink-3);white-space:nowrap}
details.goal{margin:0 0 12px}
details.goal pre{max-height:200px;overflow:auto}
.mono{font-family:var(--mono);font-size:.86em;background:var(--code);padding:1px 4px;color:#2a2620}
.scope{font-family:var(--mono);font-size:11px;color:var(--ink-3);overflow-wrap:anywhere}
.row{display:grid;grid-template-columns:88px minmax(0,1fr);gap:16px;padding:12px 0;
  border-bottom:1px solid var(--rule)}
.row .who{font-family:var(--mono);font-size:11px;color:var(--ink-2);line-height:1.5}
.row .who span{display:block;color:var(--ink-3);font-size:10px}
.row p{margin:0;max-width:80ch}
.row .scope{margin-top:5px}
details{margin-top:6px}
summary{cursor:pointer;font-family:var(--mono);font-size:11px;color:var(--ink-3)}
details pre{background:var(--code);padding:10px 12px;overflow-x:auto;font-size:12px;line-height:1.45;
  white-space:pre-wrap;margin:8px 0 0}
.card{border:1px solid var(--rule);padding:16px 18px;margin:0 0 14px;background:#fbfaf6}
.card h3{font-size:15.5px;margin:0 0 4px;letter-spacing:-.008em}
.card .head{display:flex;justify-content:space-between;gap:14px;align-items:baseline;margin-bottom:8px}
.card .sub{color:var(--ink-2);font-size:13.5px;margin:0 0 8px}
.round{border-top:1px solid var(--rule);padding:14px 0}
.round h3{font-size:15px;margin:0 0 6px}
.round .why{color:var(--ink-2);font-size:13.5px;margin:0}
.empty{color:var(--ink-3);font-size:13px}
footer{margin-top:56px;border-top:1px solid var(--rule);padding-top:12px;font-family:var(--mono);
  font-size:10.5px;color:var(--ink-3);line-height:1.7}
"""

VARIANTS = [("1-log", "Log"), ("2-by-task", "By task"), ("3-digest", "Digest first")]


def nav(current: str) -> str:
    links = []
    for slug, name in VARIANTS:
        cls = ' class="on"' if slug == current else ""
        links.append(f'<a href="{slug}.html"{cls}>{name}</a>')
    return f'<nav class="variants"><a href="index.html">Prototypes</a>{"".join(links)}</nav>'


def short_title(goal: str) -> str:
    first = goal.split(". ")[0].strip().rstrip(".")
    return first if len(first) <= 110 else first[:107].rsplit(" ", 1)[0] + "…"


def repo_root(run: dict) -> str:
    proj = run["run"].get("project") or ""
    return str(Path(proj).parent) if proj else ""


def shrink(text: str, root: str) -> str:
    if root:
        text = text.replace(root + "/", "")
    # Run-internal paths from the machine that produced the run: keep the tail, drop the prefix.
    text = re.sub(r"/(?:[\w.\-]+/)*runs/([0-9a-f\-]{36})/", r"runs/\1/", text)
    text = re.sub(r"/(?:[\w.\-]+/)+artifacts/", "artifacts/", text)
    return text


def head(run: dict) -> str:
    r, g = run["run"], run["goal"]
    status = r["status"]
    cls = "pass" if status in ("accepted", "reviewed") else "block"
    root = repo_root(run)
    project = shrink(r.get("project") or "", root)
    return f"""<header class="run">
  <div class="proj">{html.escape(project)}</div>
  <h1>{html.escape(short_title(g["text"]))}</h1>
  <p class="done">{html.escape(g.get("done") or "")}</p>
  <div class="meta">
    <span class="chip {cls}">{html.escape(status)}</span>
    <span><b>{r["rounds_used"]}</b> rounds</span>
    <span><b>{r["tokens_used"]:,}</b> tokens</span>
    <span><b>{len(run["threads"])}</b> threads</span>
    <span><b>{len(run["tasks"])}</b> tasks</span>
    <span>messages: <b>{html.escape(g["message_policy"])}</b></span>
  </div>
  <details class="goal" style="margin:10px 0 0"><summary>Full goal</summary><pre>{html.escape(g["text"])}</pre></details>
</header>"""


def checks_table(run: dict) -> str:
    root = repo_root(run)
    rows = []
    for rd in run["rounds"]:
        checks = (rd.get("readiness") or {}).get("checks") or []
        for c in checks:
            cls = "pass" if c["passed"] else "fail"
            label = "pass" if c["passed"] else "fail"
            rows.append(
                f'<tr><td class="num">round {rd["index"]}</td><td>{html.escape(c["name"])}</td>'
                f'<td class="num">{html.escape(shrink(c.get("detail") or "", root))}</td><td><span class="chip {cls}">{label}</span></td></tr>'
            )
    if not rows:
        return '<p class="empty">No checks were run.</p>'
    return ('<table><thead><tr><th>Round</th><th>Check</th><th>Detail</th><th></th></tr></thead>'
            f'<tbody>{"".join(rows)}</tbody></table>')


def artifacts_table(run: dict) -> str:
    if not run["artifacts"]:
        return '<p class="empty">No artifacts.</p>'
    root = repo_root(run)
    rows = []
    for a in run["artifacts"]:
        # In the export bundle the files live under artifacts/; show the path as it is on disk there.
        path = a["path"]
        if "/artifacts/" in path:
            rel = "artifacts/" + path.split("/artifacts/", 1)[1]
        else:
            rel = shrink(path, root)
        shown = f'<a href="{html.escape(rel)}">{html.escape(rel)}</a>' if rel.startswith("artifacts/") else html.escape(rel)
        rows.append(
            f'<tr><td>{html.escape(a["kind"])}</td><td class="scope">{shown}</td>'
            f'<td class="num">{a.get("bytes") or 0:,}</td><td class="scope">{html.escape((a.get("sha256") or "")[:12])}</td></tr>'
        )
    return ('<table><thead><tr><th>Kind</th><th>Path</th><th>Bytes</th><th>sha256</th></tr></thead>'
            f'<tbody>{"".join(rows)}</tbody></table>')


def footer(run: dict, extra: str) -> str:
    return (f'<footer>{html.escape(extra)}<br>Fixture: run {html.escape(run["run"]["id"])} · '
            f'{len(run["ledger"])} board entries · generated from the exported run, nothing invented.</footer>')


# ---------------------------------------------------------------- layout 1: log
def render_log(run: dict) -> str:
    root = repo_root(run)
    rows = []
    for e in run["ledger"]:
        scope = ", ".join(e["scope"]) if isinstance(e["scope"], list) else ""
        who = e.get("thread") or "runner"
        rnd = f'round {e["round"]}' if e.get("round") is not None else "run"
        rows.append(f"""<div class="row">
  <div class="who">{html.escape(who)}<span>{rnd}</span><span>{html.escape(e["kind"])}</span></div>
  <div><p>{html.escape(shrink(e["body"], root))}</p>{f'<div class="scope">{html.escape(shrink(scope, root))}</div>' if scope else ''}</div>
</div>""")
    body = "".join(rows) or '<p class="empty">No entries.</p>'
    return f"""{nav("1-log")}{head(run)}
<h2 class="sec">Board, oldest first</h2>
{body}
<h2 class="sec">Checks</h2>
{checks_table(run)}
<h2 class="sec">Artifacts</h2>
{artifacts_table(run)}
{footer(run, "Layout 1 — the log. Every entry in order, who wrote it, what round. Honest and noisy.")}"""


# ------------------------------------------------------------ layout 2: by task
def render_by_task(run: dict) -> str:
    root = repo_root(run)
    cards = []
    for i, t in enumerate(run["tasks"], start=1):
        scope = ", ".join(t["scope"]) if isinstance(t["scope"], list) else ""
        cls = "fail" if t["status"] == "failed" else ("pass" if t["status"] == "reported" else "block")
        msg = t.get("message") or ""
        cards.append(f"""<div class="card">
  <div class="head"><h3>Task {i} · {html.escape(t["thread"])}</h3><span class="chip {cls}">{html.escape(t["status"])}</span></div>
  <p class="sub">{html.escape(shrink(t["prompt"], root))}</p>
  <div class="scope">round {t["round"]} · created by {html.escape(t["created_by"])} · scope: {html.escape(shrink(scope, root))}</div>
  {f'<details><summary>What the agent said ({len(msg):,} chars)</summary><pre>{html.escape(shrink(msg, root))}</pre></details>' if msg else ''}
  {f'<p class="sub" style="margin-top:8px"><b>Reported:</b> {html.escape(t["result"] or "")}</p>' if t.get("result") else ''}
</div>""")
    body = "".join(cards) or '<p class="empty">No tasks.</p>'
    return f"""{nav("2-by-task")}{head(run)}
<h2 class="sec">Tasks, in the order they ran</h2>
{body}
<h2 class="sec">Checks</h2>
{checks_table(run)}
<h2 class="sec">Artifacts</h2>
{artifacts_table(run)}
{footer(run, "Layout 2 — grouped by task. Each card is one job: who, what was asked, what came back.")}"""


# -------------------------------------------------------- layout 3: digest first
def render_digest(run: dict) -> str:
    root = repo_root(run)
    rounds = []
    for rd in run["rounds"]:
        checks = (rd.get("readiness") or {}).get("checks") or []
        failed = [c["name"] for c in checks if not c["passed"]]
        done = [t for t in run["tasks"] if t["round"] == rd["index"]]
        verdict = rd.get("verdict") or "open"
        cls = "pass" if verdict == "pass" else "fail"
        why = ("every check passed" if not failed else
               "failed: " + "; ".join(failed))
        rounds.append(f"""<div class="round">
  <h3>Round {rd["index"]} <span class="chip {cls}">{html.escape(verdict)}</span></h3>
  <p class="why">{len(done)} task(s): {html.escape(", ".join(t["thread"] for t in done))} — {html.escape(why)}.</p>
</div>""")
    digest = "".join(rounds) or '<p class="empty">No rounds yet.</p>'
    rows = []
    for e in run["ledger"]:
        scope = ", ".join(e["scope"]) if isinstance(e["scope"], list) else ""
        rows.append(f"""<div class="row">
  <div class="who">{html.escape(e.get("thread") or "runner")}<span>{html.escape(e["kind"])}</span></div>
  <div><p>{html.escape(shrink(e["body"], root))}</p>{f'<div class="scope">{html.escape(shrink(scope, root))}</div>' if scope else ''}</div>
</div>""")
    return f"""{nav("3-digest")}{head(run)}
<h2 class="sec">What happened, by round</h2>
{digest}
<h2 class="sec">Checks</h2>
{checks_table(run)}
<h2 class="sec">Full board</h2>
{"".join(rows) or '<p class="empty">No entries.</p>'}
<h2 class="sec">Artifacts</h2>
{artifacts_table(run)}
{footer(run, "Layout 3 — a short digest of each round first, then the same log underneath.")}"""


INDEX = """<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Board prototypes</title><style>{css}</style></head><body><div class="wrap">
<div class="proj" style="font-family:var(--mono);font-size:11px;letter-spacing:.12em;color:var(--ink-3)">
PROTOTYPES · ONE REAL RUN, THREE ARRANGEMENTS</div>
<h1 style="font-size:30px;letter-spacing:-.02em;margin:8px 0 12px">The same board, three ways</h1>
<p style="color:var(--ink-2);max-width:70ch;margin:0 0 28px">
All three read the same exported run: {threads} threads, {tasks} tasks, {rounds} rounds,
{entries} board entries, one round that failed and one that passed. Nothing here is invented.
Pick the one you would actually want to look at.</p>
<ol style="list-style:none;padding:0;margin:0;border-top:1.2px solid var(--ink)">
<li style="border-bottom:1px solid var(--rule)"><a href="1-log.html" style="display:flex;justify-content:space-between;gap:18px;padding:18px 4px;color:inherit;border:0">
  <span><b style="font-size:17px">1 · Log</b>
  <div style="color:var(--ink-2);font-size:13.5px;margin-top:3px">Every entry in order, with who wrote it. Honest, complete, noisy.</div></span>
  <span style="font-family:var(--mono);font-size:11px;color:var(--accent);white-space:nowrap;align-self:center">open →</span></a></li>
<li style="border-bottom:1px solid var(--rule)"><a href="2-by-task.html" style="display:flex;justify-content:space-between;gap:18px;padding:18px 4px;color:inherit;border:0">
  <span><b style="font-size:17px">2 · By task</b>
  <div style="color:var(--ink-2);font-size:13.5px;margin-top:3px">One card per job: who, what was asked, what came back. Best if you care what each agent did.</div></span>
  <span style="font-family:var(--mono);font-size:11px;color:var(--accent);white-space:nowrap;align-self:center">open →</span></a></li>
<li style="border-bottom:1px solid var(--rule)"><a href="3-digest.html" style="display:flex;justify-content:space-between;gap:18px;padding:18px 4px;color:inherit;border:0">
  <span><b style="font-size:17px">3 · Digest first</b>
  <div style="color:var(--ink-2);font-size:13.5px;margin-top:3px">A short "what happened this round" at the top, then the log. Best if you want the state in ten seconds.</div></span>
  <span style="font-family:var(--mono);font-size:11px;color:var(--accent);white-space:nowrap;align-self:center">open →</span></a></li>
</ol>
<footer>Generated by tools/render-prototype.py from fixtures/board-experiment/run.json.</footer>
</div></body></html>"""


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    fixture, out = Path(argv[0]), Path(argv[1])
    run = json.loads((fixture / "run.json").read_text())
    out.mkdir(parents=True, exist_ok=True)
    for slug, render in (("1-log", render_log), ("2-by-task", render_by_task), ("3-digest", render_digest)):
        (out / f"{slug}.html").write_text(
            f'<!doctype html><html lang="en"><head><meta charset="utf-8">'
            f'<meta name="viewport" content="width=device-width, initial-scale=1">'
            f'<title>{slug} · {html.escape(run["goal"]["text"][:40])}</title>'
            f'<style>{CSS}</style></head><body><div class="wrap">{render(run)}</div></body></html>'
        )
    (out / "index.html").write_text(INDEX.format(
        css=CSS, threads=len(run["threads"]), tasks=len(run["tasks"]),
        rounds=len(run["rounds"]), entries=len(run["ledger"])))
    print(f"wrote {out}/index.html, 1-log.html, 2-by-task.html, 3-digest.html")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
