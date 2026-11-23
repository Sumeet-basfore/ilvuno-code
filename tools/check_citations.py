#!/usr/bin/env python3
"""Machine check for a brief: every section cites a section of the source, and every source section is cited.

Usage: check_citations.py BRIEF.html SOURCE.md [SECTIONS]

SECTIONS is an optional comma list (e.g. 1,2,3,4) of the source sections that must be cited. Default: all.

A citation looks like: <cite data-src="09-spike.md" data-section="3">§3</cite>
It resolves when data-src is the source's file name and data-section is a `## N.` heading in the source.

Exit 0 when the brief has at least one <section>, each section has a citation that resolves, and every
section of the source is cited somewhere. Otherwise print each problem and exit 1.
"""

import os
import re
import sys
from html.parser import HTMLParser


class BriefReader(HTMLParser):
    """Collects, per <section>, the (data-src, data-section) pairs of the <cite> tags inside it."""

    def __init__(self) -> None:
        super().__init__()
        self.sections: list[list[tuple[str, int]]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag == "section":
            self.sections.append([])
        if tag == "cite" and self.sections:
            attributes = dict(attrs)
            src, number = attributes.get("data-src"), attributes.get("data-section", "")
            if src and number.strip().isdigit():
                self.sections[-1].append((src, int(number)))


def source_sections(text: str) -> set[int]:
    return {int(n) for n in re.findall(r"^## (\d+)\.", text, re.M)}


def main(argv: list[str]) -> int:
    if len(argv) not in (3, 4):
        print(__doc__.strip(), file=sys.stderr)
        return 2
    brief_path, source_path = argv[1], argv[2]
    with open(brief_path, encoding="utf-8") as f:
        reader = BriefReader()
        reader.feed(f.read())
    with open(source_path, encoding="utf-8") as f:
        known = source_sections(f.read())
    source_name = os.path.basename(source_path)
    required = known if len(argv) == 3 else {int(n) for n in argv[3].split(",") if n.strip()}
    unknown = required - known
    if unknown:
        print(f"FAIL: required sections not in the source: {sorted(unknown)}")
        return 1

    problems = []
    if not reader.sections:
        problems.append("no <section> elements in the brief")
    cited = set()
    for index, cites in enumerate(reader.sections, start=1):
        valid = {n for src, n in cites if src == source_name and n in known}
        if not valid:
            problems.append(f"section {index} has no citation that resolves to {source_name}")
        cited |= valid
    never_cited = sorted(required - cited)
    if never_cited:
        problems.append("required source sections never cited: " + ", ".join(str(n) for n in never_cited))

    if problems:
        for problem in problems:
            print(f"FAIL: {problem}")
        return 1
    print(f"ok: {len(reader.sections)} sections, all {len(required)} required source sections cited")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
