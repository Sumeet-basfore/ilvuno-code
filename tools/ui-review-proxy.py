#!/usr/bin/env python3
"""Test proxy in front of the real runner (for checking the window against a replayed review).

Use it on a COPY of the data, never the live ILVUNO_HOME:
  ILVUNO_PORT=4738 ILVUNO_HOME=<copy> ./target/debug/ilvunod &
  python3 tools/ui-review-proxy.py &          # window port 4737 -> runner 4738
  cargo run -p desktop                          # then drive the review in the window

Listens on 127.0.0.1:4737 (the window's fixed port) and forwards to the real
ilvunod on 127.0.0.1:4738. It can overlay a run's status and one thread's
commit/stat/patch from a control file, so a review sequence can be replayed
without a provider. /events is answered 404 so the window polls instead.

Control file (default /tmp/ilvuno-review-control.json; set PROXY_CONTROL to move it). Keys (all optional):
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

UPSTREAM = "http://127.0.0.1:4738"
CONTROL = os.environ.get("PROXY_CONTROL", "/tmp/ilvuno-review-control.json")


def control():
    try:
        with open(CONTROL) as f:
            return json.load(f)
    except (OSError, ValueError):
        return {}


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        sys.stderr.write("proxy: " + (fmt % args) + "\n")

    def _send(self, status, body, ctype="application/json"):
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "content-type")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, PUT, OPTIONS")
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self):
        self._send(204, b"", "text/plain")

    def _forward(self, method):
        if self.path.split("?")[0].endswith("/events"):
            return self._send(404, b'{"error":"no events in test proxy","kind":"not_found"}')
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else None
        req = urllib.request.Request(UPSTREAM + self.path, data=body, method=method)
        if self.headers.get("Content-Type"):
            req.add_header("Content-Type", self.headers["Content-Type"])
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                status, data, ctype = resp.status, resp.read(), resp.headers.get("Content-Type", "application/json")
        except urllib.error.HTTPError as e:
            status, data, ctype = e.code, e.read(), e.headers.get("Content-Type", "application/json")
        except OSError as e:
            return self._send(502, json.dumps({"error": str(e), "kind": "internal"}).encode())

        if method == "GET" and ctype.startswith("application/json"):
            data = self._overlay(data)
        self._send(status, data, ctype)

    def _overlay(self, data):
        ctl = control()
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

    def do_GET(self):
        self._forward("GET")

    def do_POST(self):
        self._forward("POST")

    def do_PUT(self):
        self._forward("PUT")


if __name__ == "__main__":
    port = int(os.environ.get("PROXY_PORT", "4737"))
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
