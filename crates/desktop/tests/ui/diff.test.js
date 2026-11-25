// Tests for the window's patch parser and cache key (crates/desktop/ui/diff.js).
// Run: node --test crates/desktop/tests/ui/
const test = require('node:test');
const assert = require('node:assert/strict');
const D = require('../../ui/diff.js');

const ONE_EDIT = [
  'diff --git a/src/git.rs b/src/git.rs',
  'index 18411c6..a0a838a 100644',
  '--- a/src/git.rs',
  '+++ b/src/git.rs',
  '@@ -291,6 +291,8 @@ mod tests {',
  '         assert!(ok);',
  '     }',
  '+',
  '+    #[test]',
  '+    fn added() {}',
  '-    fn old() {}',
  '     // end',
  '',
].join('\n');

const NEW_FILE = [
  'diff --git a/notes.md b/notes.md',
  'new file mode 100644',
  'index 0000000..1111111',
  '--- /dev/null',
  '+++ b/notes.md',
  '@@ -0,0 +1,2 @@',
  '+# Notes',
  '+one line',
  '',
].join('\n');

const DELETED = [
  'diff --git a/old.txt b/old.txt',
  'deleted file mode 100644',
  'index 2222222..0000000',
  '--- a/old.txt',
  '+++ /dev/null',
  '@@ -1 +0,0 @@',
  '-gone',
  '',
].join('\n');

test('cache key moves when the commit moves, even if the stat text is the same', () => {
  // Regression: two rounds that each add one test show the same stat. The old key never changed.
  const thread = { name: 'writer', stat: '1 file changed, 8 insertions(+)' };
  const round1 = D.cacheKey('run-1', { ...thread, commit: '82486ce' });
  const round2 = D.cacheKey('run-1', { ...thread, commit: 'aaaaaaa' });
  assert.notEqual(round1, round2);
});

test('cache key is stable for the same commit and separate per run and thread', () => {
  assert.equal(D.cacheKey('r', { name: 't', commit: 'c1' }), D.cacheKey('r', { name: 't', commit: 'c1' }));
  assert.notEqual(D.cacheKey('r1', { name: 't', commit: 'c1' }), D.cacheKey('r2', { name: 't', commit: 'c1' }));
  assert.notEqual(D.cacheKey('r', { name: 'a', commit: 'c1' }), D.cacheKey('r', { name: 'b', commit: 'c1' }));
});

test('a thread with no commit yet still gets a usable key', () => {
  assert.equal(D.cacheKey('r', { name: 't' }), 'r/t/no-commit');
});

test('parses one modified file with counts and numbered rows', () => {
  const files = D.splitFiles(ONE_EDIT);
  assert.equal(files.length, 1);
  const [f] = files;
  assert.equal(f.path, 'src/git.rs');
  assert.equal(f.kind, 'modified');
  assert.equal(f.added, 3);
  assert.equal(f.removed, 1);
  const rows = f.hunks[0].rows;
  assert.equal(rows[0].type, 'ctx');
  assert.equal(rows[0].oldNo, 291);
  assert.equal(rows[0].newNo, 291);
  const added = rows.filter((r) => r.type === 'add');
  assert.deepEqual(added.map((r) => r.newNo), [293, 294, 295]);
  const removed = rows.find((r) => r.type === 'del');
  assert.equal(removed.oldNo, 293);
  assert.equal(removed.newNo, null);
});

test('detects added and deleted files and their paths', () => {
  const [added] = D.splitFiles(NEW_FILE);
  assert.equal(added.kind, 'added');
  assert.equal(added.path, 'notes.md');
  assert.equal(added.added, 2);

  const [deleted] = D.splitFiles(DELETED);
  assert.equal(deleted.kind, 'deleted');
  assert.equal(deleted.path, 'old.txt');
  assert.equal(deleted.removed, 1);
});

test('splits several files and totals them', () => {
  const files = D.splitFiles(ONE_EDIT + NEW_FILE + DELETED);
  assert.deepEqual(files.map((f) => f.path), ['src/git.rs', 'notes.md', 'old.txt']);
  assert.deepEqual(D.summarize(files), { files: 3, added: 5, removed: 2 });
});

test('empty or missing patch gives no files', () => {
  assert.deepEqual(D.splitFiles(''), []);
  assert.deepEqual(D.splitFiles(undefined), []);
});

test('marks binary files without inventing lines', () => {
  const [f] = D.splitFiles([
    'diff --git a/logo.png b/logo.png',
    'index 3333333..4444444 100644',
    'Binary files a/logo.png and b/logo.png differ',
    '',
  ].join('\n'));
  assert.equal(f.binary, true);
  assert.equal(f.hunks.length, 0);
});
