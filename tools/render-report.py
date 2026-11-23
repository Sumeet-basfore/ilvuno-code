#!/usr/bin/env python3
"""Render research markdown reports as self-contained HTML.

Usage:
  python3 tools/render-report.py research/*.md      # renders each .md next to itself
  python3 tools/render-report.py --index research   # writes research/index.html

CommonMark parsing (markdown_it), design matched to research/deck.html:
warm paper, one accent, mono for evidence, sticky table of contents.
No network, no template files, no new dependencies.
"""

from __future__ import annotations

import html
import re
import sys
from pathlib import Path

from markdown_it import MarkdownIt

MD = MarkdownIt("commonmark", {"html": True}).enable(["table", "strikethrough"])

CSS = """
:root{
  --paper:#f6f4ef; --paper-2:#efece4; --ink:#16150f; --ink-2:#56524a; --ink-3:#6e6a5e;
  --rule:#d8d3c7; --accent:#9e3b22; --accent-soft:#f1e5dd; --code-bg:#efece4;
  --mono:"JetBrains Mono","Liberation Mono",ui-monospace,monospace;
  --sans:"Liberation Sans","Noto Sans",Inter,system-ui,sans-serif;
}
*{box-sizing:border-box}
html{background:var(--paper)}
body{margin:0; color:var(--ink); background:var(--paper);
  font-family:var(--sans); font-size:16.5px; line-height:1.62;
  -webkit-font-smoothing:antialiased; text-rendering:optimizeLegibility}
.wrap{display:grid; grid-template-columns:248px minmax(0,1fr); gap:52px;
  max-width:1180px; margin:0 auto; padding:56px 40px 120px}
/* ---------- sidebar ---------- */
aside{position:sticky; top:40px; align-self:start; max-height:calc(100vh - 80px);
  overflow:auto; padding-left:16px; border-left:2px solid var(--accent)}
aside .kicker{font-family:var(--mono); font-size:10.5px; letter-spacing:.14em;
  text-transform:uppercase; color:var(--accent)}
aside .meta{font-family:var(--mono); font-size:11px; color:var(--ink-3); line-height:1.7; margin-top:10px}
aside nav{margin-top:22px}
aside nav a{display:block; color:var(--ink-2); text-decoration:none; font-size:13px;
  line-height:1.35; padding:4px 0 4px 10px; border-left:1px solid transparent; margin-left:-11px}
aside nav a:hover{color:var(--ink); border-left-color:var(--rule)}
aside nav a.lvl3{padding-left:22px; font-size:12.2px; color:var(--ink-3)}
aside .back{display:inline-block; margin-top:26px; font-family:var(--mono); font-size:11px;
  color:var(--ink-3); text-decoration:none; border-bottom:1px solid var(--rule)}
/* ---------- content ---------- */
main{max-width:74ch}
h1,h1 a,h2,h2 a,h3,h3 a,h4,h4 a{color:inherit}
h1{font-size:36px; line-height:1.08; letter-spacing:-.022em; margin:0 0 10px; font-weight:700}
h1 + p, h1 + blockquote{color:var(--ink-2); font-size:16.5px}
h2{font-size:23px; line-height:1.2; letter-spacing:-.014em; margin:52px 0 14px; padding-top:16px;
  border-top:1px solid var(--rule); font-weight:700}
h2:first-of-type{border-top:0; padding-top:0; margin-top:34px}
h3{font-size:17px; margin:30px 0 8px; letter-spacing:-.006em}
h4{font-size:14.5px; margin:22px 0 6px; color:var(--ink-2)}
p{margin:0 0 14px}
a{color:var(--accent); text-decoration:none; border-bottom:1px solid var(--accent-soft)}
a:hover{border-bottom-color:var(--accent)}
ul,ol{margin:0 0 16px; padding-left:22px}
li{margin:5px 0}
li::marker{color:var(--ink-3)}
hr{border:0; border-top:1px solid var(--rule); margin:38px 0}
/* a markdown --- right before a heading already gets the heading's rule */
hr:has(+ h2){display:none}
hr + h2{border-top:0; padding-top:0; margin-top:44px}
blockquote{margin:22px 0; padding:2px 0 2px 18px; border-left:2px solid var(--accent);
  color:var(--ink-2)}
blockquote p{margin:0 0 8px}
blockquote p:last-child{margin:0}
code{font-family:var(--mono); font-size:.855em; background:var(--code-bg);
  padding:1.5px 5px; border-radius:2px; color:#2a2620; overflow-wrap:break-word}
pre{background:var(--code-bg); padding:15px 17px; overflow-x:auto; margin:0 0 20px;
  border-left:2px solid var(--ink-3); line-height:1.5}
pre code{background:none; padding:0; font-size:12.6px; white-space:pre}
table{width:100%; border-collapse:collapse; margin:8px 0 26px; font-size:13.6px}
thead th{font-family:var(--mono); font-size:10.5px; letter-spacing:.1em; text-transform:uppercase;
  color:var(--ink-3); font-weight:400; text-align:left; padding:0 12px 7px 0;
  border-bottom:1.2px solid var(--ink); vertical-align:bottom}
tbody td{padding:8px 12px 8px 0; border-bottom:1px solid var(--rule); vertical-align:top; line-height:1.45}
tbody td:first-child{font-weight:700}
td code,th code{font-size:12px}
/* ---------- index ---------- */
.index{max-width:1020px; margin:0 auto; padding:64px 40px 120px}
.index .kicker{font-family:var(--mono); font-size:11px; letter-spacing:.14em;
  text-transform:uppercase; color:var(--accent)}
.index h1{margin-top:14px}
.index .lede{color:var(--ink-2); font-size:18px; max-width:62ch; margin:14px 0 38px; line-height:1.55}
.index ol{list-style:none; padding:0; margin:0; border-top:1.2px solid var(--ink)}
.index li{border-bottom:1px solid var(--rule)}
.index a.row{display:grid; grid-template-columns:34px minmax(0,1fr) 76px; gap:18px;
  align-items:start; padding:16px 4px 16px 0; text-decoration:none; border:0; color:inherit}
.index a.row:hover{background:var(--paper-2)}
.index .n{font-family:var(--mono); font-size:11.5px; color:var(--accent); white-space:nowrap;
  padding-top:4px}
.index .t{font-size:17px; font-weight:700; letter-spacing:-.008em; line-height:1.3}
.index .d{font-size:13.4px; color:var(--ink-2); margin-top:4px; line-height:1.5}
.index .w{font-family:var(--mono); font-size:11px; color:var(--ink-3); text-align:right; padding-top:4px}
@media (max-width:980px){
  .wrap{grid-template-columns:1fr; gap:0; padding:32px 20px 80px}
  aside{position:static; max-height:none; margin-bottom:28px}
  aside nav{display:none}
  .index{padding:32px 20px 80px}
  .index a.row{grid-template-columns:1fr; gap:4px}
  .index .w{text-align:left}
  table{display:block; overflow-x:auto; white-space:nowrap}
  table td,table th{white-space:normal}
}
@media print{
  aside{display:none} .wrap{grid-template-columns:1fr; max-width:100%; padding:0}
  body{font-size:11pt} h2{break-after:avoid} table,pre{break-inside:avoid}
  a{color:inherit; border:0}
}
"""

PAGE = """<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>{css}</style>
</head><body>
<div class="wrap">
<aside>
  <div class="kicker">{kind}</div>
  <div class="meta">{meta}</div>
  <nav>{toc}</nav>
  <a class="back" href="index.html">&larr; index</a>
</aside>
<main>
{body}
</main>
</div>
</body></html>
"""

INDEX = """<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>{css}</style>
</head><body>
<div class="index">
  <div class="kicker">{kicker}</div>
  <h1>{title}</h1>
  <p class="lede">{lede}</p>
  <ol>{items}</ol>
</div>
</body></html>
"""


def slugify(text: str) -> str:
    s = text.lower()
    s = re.sub(r"[`*_\[\]()]", "", s)
    s = re.sub(r"[^\w\s-]", "", s, flags=re.UNICODE)
    s = re.sub(r"[\s_]+", "-", s.strip())
    return re.sub(r"-{2,}", "-", s) or "section"


def plain(text: str) -> str:
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"[`*_]", "", text)
    return re.sub(r"\s+", " ", text).strip()


def render(path: Path) -> dict:
    raw = path.read_text(encoding="utf-8")
    tokens = MD.parse(raw)

    title = plain(path.stem)
    toc: list[str] = []
    seen: dict[str, int] = {}
    summary = ""

    for i, tok in enumerate(tokens):
        if tok.type == "heading_open" and tok.tag in ("h2", "h3"):
            inline = tokens[i + 1]
            text = plain(inline.content)
            slug = slugify(text)
            if slug in seen:
                seen[slug] += 1
                slug = f"{slug}-{seen[slug]}"
            else:
                seen[slug] = 0
            tok.attrSet("id", slug)
            cls = "lvl2" if tok.tag == "h2" else "lvl3"
            toc.append(f'<a class="{cls}" href="#{slug}">{html.escape(text)}</a>')
        elif tok.type == "heading_open" and tok.tag == "h1" and title == path.stem:
            title = plain(tokens[i + 1].content)
        elif tok.type == "paragraph_open" and not summary:
            # first prose paragraph after the title = the report's standfirst
            summary = plain(tokens[i + 1].content)

    body = MD.renderer.render(tokens, MD.options, {})
    words = len(re.findall(r"\w+", plain(raw)))
    sections = sum(1 for t in tokens if t.type == "heading_open" and t.tag == "h2")
    meta = f"{words:,} words<br>{sections} sections<br>{len(raw.splitlines())} lines of source"

    out = path.with_suffix(".html")
    kind = "Project document" if path.parent.name == "docs" else "Research report"
    out.write_text(
        PAGE.format(title=html.escape(title), css=CSS, meta=meta, kind=kind,
                    toc="\n".join(toc), body=body),
        encoding="utf-8",
    )
    return {"path": out, "title": title, "words": words, "n": path.stem,
            "summary": summary[:230] + ("…" if len(summary) > 230 else "")}


def num_prefix(stem: str) -> str:
    m = re.match(r"^(\d+)", stem)
    return m.group(1) if m else ""


def write_index(folder: Path, title: str, lede: str) -> Path:
    rows = sorted((render(md) for md in folder.glob("*.md")), key=lambda r: r["n"])
    items = "\n".join(
        f'<li><a class="row" href="{r["path"].name}">'
        f'<span class="n">{num_prefix(r["n"])}</span>'
        f'<span><span class="t">{html.escape(r["title"])}</span>'
        f'<div class="d">{html.escape(r["summary"])}</div></span>'
        f'<span class="w">{r["words"]:,}<br>words</span></a></li>'
        for r in rows
    )
    out = folder / "index.html"
    kicker = "ilvuno &middot; docs" if folder.name == "docs" else "ilvuno &middot; research"
    out.write_text(INDEX.format(title=html.escape(title), css=CSS, kicker=kicker,
                                lede=html.escape(lede), items=items), encoding="utf-8")
    return out


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__)
        return 2
    if argv[0] == "--index":
        folder = Path(argv[1])
        title = argv[2] if len(argv) > 2 else "Reports"
        lede = argv[3] if len(argv) > 3 else ""
        out = write_index(folder, title, lede)
        print(f"wrote {out}")
        return 0
    for arg in argv:
        p = Path(arg)
        if p.suffix == ".md":
            r = render(p)
            print(f"wrote {r['path']}  ({r['words']:,} words)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
