#!/usr/bin/env python3
"""Test proxy in front of the real runner, for checking the window against a replayed review.

Use it on a COPY of the data, never the live ILVUNO_HOME:

  ILVUNO_PORT=4738 ILVUNO_HOME=<copy> ./target/debug/ilvunod &
  python3 tools/ui-review-proxy.py &          # window port 4737 -> runner 4738
  cargo run -p desktop                          # then drive the review in the window

Listens on 127.0.0.1:4737 (the window's fixed port) and forwards to the runner on 127.0.0.1:4738.
It can overlay a run's status and one thread's commit, stat and patch from a control file, so a
review sequence can be replayed without a provider. /events is answered 404 so the window polls.

Security: the runner only trusts the window's own origins. This proxy keeps the same rule. A request
with an Origin header that is not an allowed window origin gets 403 and never reaches the runner, and
the allowed origin is echoed back, never "*". The Origin header is forwarded upstream so the runner
applies its own checks to the real caller.

Control file (default /tmp/ilvuno-review-control.json; set PROXY_CONTROL to move it). Keys, all optional:
  run_status   string   overrides run.status (GET /runs/:id and /runs)
  thread       string   thread to overlay (default writer)
  commit       string   overrides that thread's commit
  stat         string   overrides that thread's stat
  patch_file   string   path to a file whose text replaces that thread's diff patch
"""

import json
import os
import sys
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Optional

UPSTREAM = "http://127.0.0.1:4738"
CONTROL = os.environ.get("PROXY_CONTROL", "/tmp/ilvuno-review-control.json")
# The desktop window's origins (docs/api.md). Nothing else may use this proxy from a browser.
ALLOWED_ORIGINS = frozenset({"tauri://localhost", "http://tauri.localhost"})
FORBIDDEN = b'{"error":"origin not allowed","kind":"forbidden"}'
BAD_HOST = b'{"error":"host not allowed","kind":"forbidden"}'
# DNS rebinding: a hostile page can make its own name resolve to 127.0.0.1, and then its requests are
# same-origin with no Origin header. Only requests addressed to this proxy's own loopback name pass.
PORT = int(os.environ.get("PROXY_PORT", "4737"))
ALLOWED_HOSTS = frozenset({f"127.0.0.1:{PORT}", f"localhost:{PORT}"})


def read_control() -> dict[str, Any]:
    try:
        with open(CONTROL, encoding="utf-8") as f:
            value = json.load(f)
    except (OSError, ValueError):
        return {}
    return value if isinstance(value, dict) else {}


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt: str, *args: Any) -> None:
        sys.stderr.write("proxy: " + (fmt % args) + "\n")

    def _origin(self) -> Optional[str]:
        return self.headers.get("Origin")

    def _origin_allowed(self) -> bool:
        # No Origin header means a local program (the runner allows those too). A browser page always
        # sends one, so only the window's origins pass.
        origin = self._origin()
        return origin is None or origin in ALLOWED_ORIGINS

    def _send(self, status: int, body: bytes, ctype: str = "application/json") -> None:
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        origin = self._origin()
        if origin in ALLOWED_ORIGINS:
            # Echo the one allowed origin. A wildcard would let any web page read runs.
            self.send_header("Access-Control-Allow-Origin", origin)
            self.send_header("Vary", "Origin")
            self.send_header("Access-Control-Allow-Headers", "content-type")
            self.send_header("Access-Control-Allow-Methods", "GET, POST, PUT, OPTIONS")
        self.end_headers()
        self.wfile.write(body)

    def _host_allowed(self) -> bool:
        return self.headers.get("Host") in ALLOWED_HOSTS

    def do_OPTIONS(self) -> None:
        if not self._host_allowed():
            return self._send(403, BAD_HOST)
        if not self._origin_allowed():
            return self._send(403, FORBIDDEN)
        self._send(204, b"", "text/plain")

    def _forward(self, method: str) -> None:
        if not self._host_allowed():
            return self._send(403, BAD_HOST)
        if not self._origin_allowed():
            return self._send(403, FORBIDDEN)
        if self.path.split("?")[0].endswith("/events"):
            return self._send(404, b'{"error":"no events in test proxy","kind":"not_found"}')
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else None
        req = urllib.request.Request(UPSTREAM + self.path, data=body, method=method)
        if self.headers.get("Content-Type"):
            req.add_header("Content-Type", self.headers["Content-Type"])
        if self._origin():
            req.add_header("Origin", self._origin() or "")
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                status, data = resp.status, resp.read()
                ctype = resp.headers.get("Content-Type", "application/json")
        except urllib.error.HTTPError as e:
            status, data = e.code, e.read()
            ctype = e.headers.get("Content-Type", "application/json")
        except OSError as e:
            return self._send(502, json.dumps({"error": str(e), "kind": "internal"}).encode())

        if method == "GET" and ctype.startswith("application/json"):
            data = self._overlay(data)
        self._send(status, data, ctype)

    def _overlay(self, data: bytes) -> bytes:
        ctl = read_control()
        if not ctl:
            return data
        path = self.path.split("?")[0]
        thread = ctl.get("thread", "writer")
        try:
            value = json.loads(data)
        except ValueError:
            return data
        if path == "/runs":
            for run in value:
                if "run_status" in ctl:
                    run["status"] = ctl["run_status"]
            return json.dumps(value).encode()
        if path.startswith("/runs/") and path.count("/") == 2:
            if "run_status" in ctl:
                value["run"]["status"] = ctl["run_status"]
            for t in value.get("threads", []):
                if t.get("name") == thread:
                    if "commit" in ctl:
                        t["commit"] = ctl["commit"]
                    if "stat" in ctl:
                        t["stat"] = ctl["stat"]
            return json.dumps(value).encode()
        if path.endswith("/diff") and "patch_file" in ctl and f"/threads/{thread}/diff" in path:
            with open(ctl["patch_file"], encoding="utf-8") as f:
                value["patch"] = f.read()
            value["truncated"] = False
            return json.dumps(value).encode()
        return data

    def do_GET(self) -> None:
        self._forward("GET")

    def do_POST(self) -> None:
        self._forward("POST")

    def do_PUT(self) -> None:
        self._forward("PUT")


if __name__ == "__main__":
    port = int(os.environ.get("PROXY_PORT", "4737"))
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
