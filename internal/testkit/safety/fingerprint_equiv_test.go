package safety

import (
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// takeSerial is the original serial implementation of Take, written out as sequential git calls. It is
// the oracle that the concurrent Take must match field for field, and the first error it reports must be
// the one the concurrent Take reports. It shares only the leaf helpers that the optimization leaves
// unchanged: git, gitOptional, statusEntry, contentState, records, lines and digest.
func takeSerial(dir string) (Fingerprint, error) {
	fp := Fingerprint{Refs: map[string]string{}, Index: map[string]string{}, Worktree: map[string]string{}}

	refs, err := git(dir, "for-each-ref", "--format=%(refname) %(objectname)")
	if err != nil {
		return fp, err
	}
	for _, line := range lines(refs) {
		name, sha, ok := strings.Cut(line, " ")
		if !ok {
			return fp, fmt.Errorf("malformed for-each-ref line %q", line)
		}
		if !strings.HasPrefix(name, ownedRefPrefix) {
			fp.Refs[name] = sha
		}
	}

	head, err := gitOptional(dir, "rev-parse", "--verify", "-q", "HEAD")
	if err != nil {
		return fp, err
	}
	fp.Head = head
	ref, err := gitOptional(dir, "symbolic-ref", "-q", "HEAD")
	if err != nil {
		return fp, err
	}
	fp.HeadRef = ref

	idx, err := git(dir, "ls-files", "--stage", "-z")
	if err != nil {
		return fp, err
	}
	for _, rec := range records(idx) {
		meta, path, ok := strings.Cut(rec, "\t")
		if !ok {
			return fp, fmt.Errorf("malformed ls-files record %q", rec)
		}
		if strings.HasPrefix(path, ownedDir) {
			continue
		}
		f := strings.Fields(meta)
		if len(f) != 3 {
			return fp, fmt.Errorf("malformed ls-files metadata %q", meta)
		}
		key := path
		if f[2] != "0" {
			key = fmt.Sprintf("%s (stage %s)", path, f[2])
		}
		fp.Index[key] = f[0] + " " + f[1]
	}

	st, err := git(dir, "status", "--porcelain=v2", "-z", "--untracked-files=all")
	if err != nil {
		return fp, err
	}
	recs := records(st)
	for i := 0; i < len(recs); i++ {
		xy, path, renamed, ok := statusEntry(recs[i])
		if !ok {
			if strings.HasPrefix(recs[i], "# ") || strings.HasPrefix(recs[i], "! ") {
				continue
			}
			return fp, fmt.Errorf("unrecognized status record %q", recs[i])
		}
		if renamed {
			i++
		}
		if strings.HasPrefix(path, ownedDir) {
			continue
		}
		state, err := contentState(dir, path)
		if err != nil {
			return fp, err
		}
		fp.Worktree[path] = xy + " " + state
	}

	stashes, err := git(dir, "stash", "list", "--format=%H")
	if err != nil {
		return fp, err
	}
	fp.Stashes = lines(stashes)

	p, err := git(dir, "rev-parse", "--git-path", "config")
	if err != nil {
		return fp, err
	}
	if !filepath.IsAbs(p) {
		p = filepath.Join(dir, p)
	}
	b, err := os.ReadFile(p)
	if err != nil {
		return fp, fmt.Errorf("read config: %w", err)
	}
	fp.Config = digest(b)
	return fp, nil
}

// equivCase builds one representative repository state. dirty marks states that must record some
// worktree or index entry, so that an empty comparison cannot pass by accident.
type equivCase struct {
	name  string
	dirty bool
	build func(t *testing.T) string
}

func equivCases() []equivCase {
	return []equivCase{
		{"clean", false, func(t *testing.T) string { return baseline(t).Path }},
		{"unstaged, staged, untracked, deleted and mode changes", true, func(t *testing.T) string {
			r := baseline(t)
			r.WriteFile("shared.txt", "line1\nchanged\n")
			r.WriteFile("staged.txt", "s\n")
			r.MustGit("add", "staged.txt")
			r.WriteFile("new.txt", "n\n")
			r.RemoveFile("stable.txt")
			r.Chmod("shared.txt", 0o755)
			return r.Path
		}},
		{"staged rename", true, func(t *testing.T) string {
			r := baseline(t)
			r.MustGit("mv", "stable.txt", "renamed.txt")
			return r.Path
		}},
		{"quoted name and untracked symlink", true, func(t *testing.T) string {
			r := baseline(t)
			r.WriteFile(`odd "name" x.txt`, "a\n")
			must(os.Symlink("shared.txt", filepath.Join(r.Path, "link")))
			return r.Path
		}},
		{"detached HEAD with a dirty file", true, func(t *testing.T) string {
			r := baseline(t)
			r.MustGit("checkout", "--detach", "HEAD")
			r.WriteFile("shared.txt", "line1\ndetached\n")
			return r.Path
		}},
		{"unborn HEAD with a staged file", true, func(t *testing.T) string {
			r := forge.New(t)
			r.WriteFile("a.txt", "a\n")
			r.MustGit("add", "a.txt")
			return r.Path
		}},
		{"stash present", false, func(t *testing.T) string {
			r := baseline(t)
			r.WriteFile("shared.txt", "stashed\n")
			r.MustGit("stash", "push", "-m", "t")
			return r.Path
		}},
		{"candidate namespace and .ilvuno present", false, func(t *testing.T) string {
			r := baseline(t)
			r.MustGit("update-ref", "refs/ilvuno/candidates/c1", "HEAD")
			r.WriteFile(".ilvuno/candidate/state.json", "{}\n")
			return r.Path
		}},
	}
}

// TestConcurrentTakeMatchesSerialOracle compares the concurrent Take with the serial oracle on every
// representative state, repeatedly, so that a scheduling-dependent difference would show up.
func TestConcurrentTakeMatchesSerialOracle(t *testing.T) {
	for _, tc := range equivCases() {
		t.Run(tc.name, func(t *testing.T) {
			dir := tc.build(t)
			want, wantErr := takeSerial(dir)
			if wantErr != nil {
				t.Fatalf("serial oracle failed on a valid state: %v", wantErr)
			}
			if tc.dirty && len(want.Worktree)+len(want.Index) == 0 {
				t.Fatal("dirty state recorded no index or worktree entry; the comparison would be vacuous")
			}
			for i := 0; i < 5; i++ {
				got, err := Take(dir)
				if err != nil {
					t.Fatalf("Take failed on run %d: %v", i, err)
				}
				if !reflect.DeepEqual(want, got) {
					t.Fatalf("run %d: concurrent Take differs from the serial oracle\nserial:     %+v\nconcurrent: %+v", i, want, got)
				}
			}
		})
	}
}

// TestConcurrentTakeReportsTheSameFirstError: when a read fails, the concurrent Take returns the same
// error, and the same partial fingerprint, as the serial oracle, even though every read has already run.
func TestConcurrentTakeReportsTheSameFirstError(t *testing.T) {
	cases := []struct {
		name  string
		build func(t *testing.T) string
	}{
		{"not a work tree", func(t *testing.T) string { return t.TempDir() }},
		{"corrupt index", func(t *testing.T) string {
			r := baseline(t)
			must(os.WriteFile(filepath.Join(r.Path, ".git", "index"), []byte("garbage"), 0o644))
			return r.Path
		}},
		{"unreadable modified file", func(t *testing.T) string {
			if os.Geteuid() == 0 {
				t.Skip("root reads any file, so the read cannot fail")
			}
			r := baseline(t)
			r.WriteFile("shared.txt", "line1\nchanged and longer\n")
			path := filepath.Join(r.Path, "shared.txt")
			must(os.Chmod(path, 0o000))
			t.Cleanup(func() { _ = os.Chmod(path, 0o644) })
			return r.Path
		}},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			dir := tc.build(t)
			want, wantErr := takeSerial(dir)
			got, gotErr := Take(dir)
			if wantErr == nil || gotErr == nil {
				t.Fatalf("expected both to fail: serial err=%v, concurrent err=%v", wantErr, gotErr)
			}
			if wantErr.Error() != gotErr.Error() {
				t.Fatalf("first error differs\nserial:     %v\nconcurrent: %v", wantErr, gotErr)
			}
			if !reflect.DeepEqual(want, got) {
				t.Fatalf("partial fingerprint differs\nserial:     %+v\nconcurrent: %+v", want, got)
			}
		})
	}
}

// TestTakeLeavesNoGoroutinesBehind: every goroutine started by Take has finished when Take returns.
func TestTakeLeavesNoGoroutinesBehind(t *testing.T) {
	r := baseline(t)
	before := runtime.NumGoroutine()
	for i := 0; i < 20; i++ {
		takeT(t, r.Path)
	}
	deadline := time.Now().Add(2 * time.Second)
	for runtime.NumGoroutine() > before && time.Now().Before(deadline) {
		time.Sleep(10 * time.Millisecond)
	}
	if n := runtime.NumGoroutine(); n > before {
		t.Fatalf("goroutines grew from %d to %d after 20 Take calls", before, n)
	}
}
