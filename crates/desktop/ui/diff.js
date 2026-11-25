// Parse a unified git patch into files and lines, and key a cached patch by its commit.
//
// Pure functions with no DOM, so Node can test them (crates/desktop/tests/ui). The window fetches one
// patch per thread from GET /runs/:id/threads/:name/diff. A patch describes one commit, so the cache
// key includes the commit: a new round moves the commit, and the old patch is never shown again.
(function (root) {
  'use strict';

  // The cache key for one thread's patch. A thread without a commit yet gets its own key.
  function cacheKey(runId, thread) {
    return `${runId}/${thread.name}/${thread.commit || 'no-commit'}`;
  }

  // "diff --git a/x b/x" → { oldPath: 'x', newPath: 'x' }. Quoted paths keep their quotes.
  function headerPaths(line) {
    const m = /^diff --git (?:"?a\/(.*?)"?) (?:"?b\/(.*?)"?)$/.exec(line);
    if (m) return { oldPath: m[1], newPath: m[2] };
    const parts = line.replace(/^diff --git /, '').split(' ');
    const strip = (p) => (p || '').replace(/^[ab]\//, '');
    return { oldPath: strip(parts[0]), newPath: strip(parts[parts.length - 1]) };
  }

  // Split a whole patch into one parsed file per "diff --git" block.
  function splitFiles(patch) {
    const text = String(patch || '');
    if (!text.trim()) return [];
    const blocks = [];
    let current = null;
    for (const line of text.split('\n')) {
      if (line.startsWith('diff --git ')) {
        current = { header: line, lines: [] };
        blocks.push(current);
      } else if (current) {
        current.lines.push(line);
      }
    }
    return blocks.map(parseFile);
  }

  function parseFile(block) {
    const { oldPath, newPath } = headerPaths(block.header);
    let kind = 'modified';
    let binary = false;
    let added = 0;
    let removed = 0;
    const hunks = [];
    let hunk = null;
    let oldNo = 0;
    let newNo = 0;
    for (const line of block.lines) {
      if (!hunk) {
        if (line.startsWith('new file mode')) kind = 'added';
        else if (line.startsWith('deleted file mode')) kind = 'deleted';
        else if (line.startsWith('rename from')) kind = 'renamed';
        else if (line.startsWith('Binary files') || line.startsWith('GIT binary patch')) binary = true;
      }
      const h = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/.exec(line);
      if (h) {
        oldNo = Number(h[1]);
        newNo = Number(h[2]);
        hunk = { header: line, context: h[3].trim(), rows: [] };
        hunks.push(hunk);
        continue;
      }
      if (!hunk) continue;
      if (line.startsWith('\\')) {
        hunk.rows.push({ type: 'meta', oldNo: null, newNo: null, text: line });
      } else if (line.startsWith('+')) {
        added += 1;
        hunk.rows.push({ type: 'add', oldNo: null, newNo: newNo++, text: line.slice(1) });
      } else if (line.startsWith('-')) {
        removed += 1;
        hunk.rows.push({ type: 'del', oldNo: oldNo++, newNo: null, text: line.slice(1) });
      } else if (line.startsWith(' ')) {
        hunk.rows.push({ type: 'ctx', oldNo: oldNo++, newNo: newNo++, text: line.slice(1) });
      }
    }
    const path = kind === 'deleted' ? oldPath : newPath;
    return { path, oldPath, newPath, kind, binary, added, removed, hunks };
  }

  // Totals for a summary line: files, added lines, removed lines.
  function summarize(files) {
    return {
      files: files.length,
      added: files.reduce((n, f) => n + f.added, 0),
      removed: files.reduce((n, f) => n + f.removed, 0),
    };
  }

  const api = { cacheKey, headerPaths, splitFiles, parseFile, summarize };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  root.IlvunoDiff = api;
})(typeof globalThis !== 'undefined' ? globalThis : this);
