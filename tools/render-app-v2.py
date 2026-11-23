#!/usr/bin/env python3
"""Render the v2 app mock from an exported run.

    python3 tools/render-app-v2.py fixtures/board-experiment mock/v2
    python3 tools/render-app-v2.py fixtures/board-experiment/run.json mock/v2/app.html

The template lives in tools/app-v2-template.html. The run JSON is embedded in
the page (escaped for a <script> element) and the page renders from it, so the
mock never repeats the data by hand.

States live in the URL hash, read on load and on hashchange:

    #board (default) #drawer #palette #settings #settings-agents
    #settings-defaults #settings-about #offline #empty #blocked #gate
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TEMPLATE = HERE / "app-v2-template.html"
PLACEHOLDER = "__RUN_JSON__"
REQUIRED = ("run", "goal", "threads", "rounds", "tasks", "artifacts")


def fail(message: str) -> None:
    print(f"render-app-v2: {message}", file=sys.stderr)
    raise SystemExit(1)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        fail("usage: render-app-v2.py <fixture-dir|run.json> <out-dir|out.html>")

    fixture = Path(argv[1])
    if fixture.is_dir():
        fixture = fixture / "run.json"
    if not fixture.is_file():
        fail(f"no run.json at {fixture}")

    try:
        data = json.loads(fixture.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"{fixture} is not valid JSON: {exc}")

    missing = [key for key in REQUIRED if key not in data]
    if missing:
        fail(f"{fixture} is missing {', '.join(missing)}")

    if not TEMPLATE.is_file():
        fail(f"no template at {TEMPLATE}")

    payload = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    # `<` appears in real goal text ("one <section> for each source"). Escaping
    # it keeps the payload valid JSON and impossible to close the script tag.
    payload = payload.replace("<", "\\u003c")

    template = TEMPLATE.read_text(encoding="utf-8")
    if template.count(PLACEHOLDER) != 1:
        fail(f"{TEMPLATE} must contain {PLACEHOLDER} exactly once")

    out = Path(argv[2])
    if out.suffix.lower() != ".html":
        out = out / "app.html"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(template.replace(PLACEHOLDER, payload), encoding="utf-8")

    run = data["run"]
    print(
        f"wrote {out} · {out.stat().st_size:,} bytes · "
        f"run {str(run.get('id', ''))[:8]} · {run.get('status', 'unknown')}"
    )
    return 0


if __name__ == "__main__":
    main(sys.argv)
