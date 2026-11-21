#!/usr/bin/env python3
"""Emit one result record as JSON. Scalars are base64-encoded key/value lines;
arrays are NUL-separated files named <key>.nul. Guarantees valid JSON with
lossless path bytes (surrogate-escaped when not UTF-8)."""
import base64, json, os, sys

out = sys.argv[1]
rec = {}
kv = os.path.join(out, "result.kv")
with open(kv, "r", encoding="ascii") as f:
    for line in f:
        line = line.rstrip("\n")
        if not line:
            continue
        k, b = line.split("\t", 1)
        raw = base64.b64decode(b)
        rec[k] = raw.decode("utf-8", "surrogateescape")
ARRAYS = {"content", "conflict_paths", "steps"}
for name in sorted(os.listdir(out)):
    if not name.endswith(".nul") or name[:-4] not in ARRAYS:
        continue
    key = name[:-4]
    data = open(os.path.join(out, name), "rb").read()
    rec[key] = [p.decode("utf-8", "surrogateescape") for p in data.split(b"\0") if p]
NUM = {"exit", "conflict_step", "check_rc", "refs_left", "worktrees_left",
       "objects_before", "objects_after"}
for k in NUM:
    if k in rec:
        try:
            rec[k] = int(rec[k])
        except (TypeError, ValueError):
            pass
json.dump(rec, sys.stdout, ensure_ascii=True, sort_keys=True)
sys.stdout.write("\n")
