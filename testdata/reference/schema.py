#!/usr/bin/env python3
"""Strict fixture-manifest schema (single source of truth).

Format (no shell evaluation, no continuations, no comments supported):
    FORMAT=1
    BASE=<40|64 lowercase hex>
    SRCS=<oid>[ <oid>[ <oid>]]      # 2..3, unique, none equal to BASE
    CHECK_SHA256=<64 lowercase hex> # optional

  schema.py --check <file>     exit 0 valid, 1 invalid (message on stderr)
  schema.py --shell <file>     valid -> "KEY=value" lines on stdout
"""
import re, sys

OID = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
SHA = re.compile(r"^[0-9a-f]{64}$")
ALLOWED = {"FORMAT", "BASE", "SRCS", "CHECK_SHA256"}

class ManifestError(ValueError):
    pass

def parse_manifest(text):
    fields = {}
    for n, line in enumerate(text.splitlines(), 1):
        if not line:
            raise ManifestError(f"line {n}: blank line")
        if "=" not in line:
            raise ManifestError(f"line {n}: not KEY=VALUE")
        key, _, val = line.partition("=")
        if key not in ALLOWED:
            raise ManifestError(f"line {n}: unknown key {key!r}")
        if key in fields:
            raise ManifestError(f"line {n}: duplicate key {key!r}")
        if key == "FORMAT":
            if val != "1":
                raise ManifestError(f"line {n}: unsupported FORMAT {val!r}")
        elif key == "BASE":
            if not OID.match(val):
                raise ManifestError(f"line {n}: invalid BASE")
        elif key == "CHECK_SHA256":
            if not SHA.match(val):
                raise ManifestError(f"line {n}: invalid CHECK_SHA256")
        else:  # SRCS
            parts = val.split(" ")
            if any(not OID.match(p) for p in parts):
                raise ManifestError(f"line {n}: invalid SRCS entry")
            if not 2 <= len(parts) <= 3:
                raise ManifestError(f"line {n}: SRCS needs 2 or 3 entries")
            if len(set(parts)) != len(parts):
                raise ManifestError(f"line {n}: duplicate source")
            fields[key] = parts
            continue
        fields[key] = val
    for required in ("FORMAT", "BASE", "SRCS"):
        if required not in fields:
            raise ManifestError(f"missing {required}")
    if any(s == fields["BASE"] for s in fields["SRCS"]):
        raise ManifestError("source equals BASE")
    return {"FORMAT": fields["FORMAT"], "BASE": fields["BASE"],
            "SRCS": fields["SRCS"], "CHECK_SHA256": fields.get("CHECK_SHA256")}

def read_manifest_file(path):
    with open(path, "rb") as f:
        raw = f.read()
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as e:
        raise ManifestError(f"not UTF-8: {e}")
    return parse_manifest(text)

def main():
    mode, path = sys.argv[1], sys.argv[2]
    try:
        m = read_manifest_file(path)
    except (ManifestError, OSError) as e:
        print(f"manifest: {e}", file=sys.stderr)
        return 1
    if mode == "--check":
        return 0
    if mode == "--shell":
        print(f"FORMAT={m['FORMAT']}")
        print(f"BASE={m['BASE']}")
        print("SRCS=" + " ".join(m["SRCS"]))
        if m["CHECK_SHA256"]:
            print(f"CHECK_SHA256={m['CHECK_SHA256']}")
        return 0
    print("usage: schema.py --check|--shell <file>", file=sys.stderr)
    return 2

if __name__ == "__main__":
    sys.exit(main())
