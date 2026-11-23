#!/usr/bin/env python3
"""Render the v3 app mock from an exported run.

    python3 tools/render-app-v3.py fixtures/board-experiment mock/v3
    python3 tools/render-app-v3.py fixtures/board-experiment/run.json mock/v3/app.html

The template lives in tools/app-v3-template.html. Three things are injected:

- the run JSON (escaped for a <script> element),
- the fixture's raw board.md, shown in the Log tab,
- the four Inter woff2 files as base64 data URLs, so app.html is one file.

States live in the URL hash, read on load and on hashchange:

    #board (default) #tasks #log #drawer #task-N #palette #settings
    #settings-agents #settings-defaults #settings-about
    #offline #empty #blocked #gate #newrun
"""

from __future__ import annotations

import base64
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TEMPLATE = HERE / "app-v3-template.html"
FONT_DIR = HERE.parent / "assets" / "fonts"
REQUIRED = ("run", "goal", "threads", "rounds", "tasks", "ledger", "artifacts")

FONTS = (
    ("Inter", "Inter-Regular.woff2", 400),
    ("Inter", "Inter-Medium.woff2", 500),
    ("Inter", "Inter-SemiBold.woff2", 600),
    ("Inter Display", "InterDisplay-Bold.woff2", 700),
)


def fail(message: str) -> None:
    print(f"render-app-v3: {message}", file=sys.stderr)
    raise SystemExit(1)


def font_css() -> str:
    rules = []
    for family, filename, weight in FONTS:
        path = FONT_DIR / filename
        if not path.is_file():
            fail(f"missing font {path}")
        data = base64.b64encode(path.read_bytes()).decode("ascii")
        rules.append(
            "@font-face{font-family:'%s';"
            "src:url(data:font/woff2;base64,%s) format('woff2');"
            "font-weight:%d;font-style:normal;font-display:block}" % (family, data, weight)
        )
    return "\n".join(rules)


def script_json(value: object) -> str:
    # `<` appears in real goal text ("one <section> for each source"). Escaping
    # it keeps the payload valid JSON and impossible to close the script tag.
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).replace("<", "\\u003c")


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        fail("usage: render-app-v3.py <fixture-dir|run.json> <out-dir|out.html>")

    fixture = Path(argv[1])
    if fixture.is_dir():
        run_file, board_file = fixture / "run.json", fixture / "board.md"
    else:
        run_file, board_file = fixture, fixture.parent / "board.md"
    if not run_file.is_file():
        fail(f"no run.json at {run_file}")
    if not board_file.is_file():
        fail(f"no board.md next to {run_file}; the Log tab needs the raw board")

    try:
        data = json.loads(run_file.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"{run_file} is not valid JSON: {exc}")

    missing = [key for key in REQUIRED if key not in data]
    if missing:
        fail(f"{run_file} is missing {', '.join(missing)}")

    board_md = board_file.read_text(encoding="utf-8")
    template = TEMPLATE.read_text(encoding="utf-8")
    for placeholder in ("__FONT_CSS__", "__RUN_JSON__", "__BOARD_MD__"):
        if template.count(placeholder) != 1:
            fail(f"{TEMPLATE} must contain {placeholder} exactly once")

    html = template.replace("__FONT_CSS__", font_css())
    html = html.replace("__RUN_JSON__", script_json(data))
    html = html.replace("__BOARD_MD__", script_json(board_md))

    out = Path(argv[2])
    if out.suffix.lower() != ".html":
        out = out / "app.html"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(html, encoding="utf-8")

    run = data["run"]
    print(
        f"wrote {out} · {out.stat().st_size:,} bytes · "
        f"run {str(run.get('id', ''))[:8]} · {run.get('status', 'unknown')} · "
        f"{len(data['ledger'])} board entries"
    )
    return 0


if __name__ == "__main__":
    main(sys.argv)
