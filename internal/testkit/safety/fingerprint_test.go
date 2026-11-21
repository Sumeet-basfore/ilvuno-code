package safety

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// baseline builds the repository every case starts from: two files on main, and an idle branch s1 at
// the same commit. Each mutation then changes exactly one field.
func baseline(t *testing.T) *forge.Repo {
	t.Helper()
	r := forge.New(t)
	r.WriteFile("shared.txt", "line1\nline2\n")
	r.WriteFile("stable.txt", "base\n")
	r.Commit("base")
	r.Branch("s1", "main")
	r.Checkout("main")
	return r
}

func takeT(t *testing.T, dir string) Fingerprint {
	t.Helper()
	fp, err := Take(dir)
	if err != nil {
		t.Fatalf("Take(%s): %v", dir, err)
	}
	return fp
}

// contains reports whether some diff line includes want.
func contains(diff []string, want string) bool {
	for _, line := range diff {
		if strings.Contains(line, want) {
			return true
		}
	}
	return false
}

// TestMutationsAreDetectedAndNamed is AC-1 and the mutation table of the evidence requirements. Each
// case changes one field and asserts that the diff names it.
func TestMutationsAreDetectedAndNamed(t *testing.T) {
	cases := []struct {
		name   string
		setup  func(r *forge.Repo) // runs after the baseline commit and before the first snapshot
		mutate func(r *forge.Repo) // runs once, after the first snapshot
		want   string              // substring the diff must contain: the named field
	}{
		{"ref move", nil, func(r *forge.Repo) {
			r.Checkout("s1")
			r.WriteFile("s1.txt", "x\n")
			r.Commit("s1 advances")
			r.Checkout("main")
		}, "ref refs/heads/s1"},
		{"new ref", nil, func(r *forge.Repo) {
			r.MustGit("update-ref", "refs/heads/extra", "HEAD")
		}, "ref refs/heads/extra"},
		{"branch delete", nil, func(r *forge.Repo) {
			r.MustGit("branch", "-D", "s1")
		}, "ref refs/heads/s1"},
		{"staged change", nil, func(r *forge.Repo) {
			r.WriteFile("staged.txt", "s\n")
			r.MustGit("add", "staged.txt")
		}, "index staged.txt"},
		{"dirty file", nil, func(r *forge.Repo) {
			r.WriteFile("shared.txt", "line1\nchanged\n")
		}, "worktree shared.txt"},
		{"dirty file with a quoted name", func(r *forge.Repo) {
			r.WriteFile(`odd "name" x.txt`, "a\n")
			r.Commit("odd name")
		}, func(r *forge.Repo) {
			r.WriteFile(`odd "name" x.txt`, "b\n")
		}, `worktree odd "name" x.txt`},
		{"untracked file", nil, func(r *forge.Repo) {
			r.WriteFile("new.txt", "n\n")
		}, "worktree new.txt"},
		{"deleted tracked file", nil, func(r *forge.Repo) {
			r.RemoveFile("stable.txt")
		}, "worktree stable.txt"},
		{"mode change", nil, func(r *forge.Repo) {
			r.Chmod("shared.txt", 0o755)
		}, "worktree shared.txt"},
		{"symlink retarget", func(r *forge.Repo) {
			must(os.Symlink("shared.txt", filepath.Join(r.Path, "link")))
		}, func(r *forge.Repo) {
			must(os.Remove(filepath.Join(r.Path, "link")))
			must(os.Symlink("stable.txt", filepath.Join(r.Path, "link")))
		}, "worktree link"},
		{"stash", nil, func(r *forge.Repo) {
			r.WriteFile("shared.txt", "stashed\n")
			r.MustGit("stash", "push", "-m", "t")
		}, "stash:"},
		{"config edit", nil, func(r *forge.Repo) {
			r.MustGit("config", "user.name", "Changed")
		}, "config:"},
		{"symbolic HEAD only", nil, func(r *forge.Repo) {
			r.MustGit("symbolic-ref", "HEAD", "refs/heads/s1")
		}, "HEAD ref:"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			r := baseline(t)
			if tc.setup != nil {
				tc.setup(r)
			}
			before := takeT(t, r.Path)

			tc.mutate(r)
			diff := Diff(before, takeT(t, r.Path))

			if !contains(diff, tc.want) {
				t.Fatalf("diff %q does not name %q", diff, tc.want)
			}
			t.Logf("mutation %-30s -> %s", tc.name, strings.Join(diff, " | "))
		})
	}
}

// TestCandidateNamespaceIsNotAFalsePositive is AC-2: refs/ilvuno/* and .ilvuno/ are excluded by design.
func TestCandidateNamespaceIsNotAFalsePositive(t *testing.T) {
	r := baseline(t)
	before := takeT(t, r.Path)

	r.MustGit("update-ref", "refs/ilvuno/candidates/c1", "HEAD")
	r.WriteFile(".ilvuno/candidate/state.json", "{}\n")

	if d := Diff(before, takeT(t, r.Path)); len(d) != 0 {
		t.Fatalf("candidate creation reported as a change: %q", d)
	}
}

// TestDiffNamesFieldAndBothValues is AC-3: a changed ref shows the field and both object names.
func TestDiffNamesFieldAndBothValues(t *testing.T) {
	r := baseline(t)
	before := takeT(t, r.Path)
	oldSHA := before.Refs["refs/heads/s1"]

	r.Checkout("s1")
	r.WriteFile("s1.txt", "x\n")
	newSHA := r.Commit("advance")
	r.Checkout("main")

	diff := Diff(before, takeT(t, r.Path))
	var line string
	for _, l := range diff {
		if strings.Contains(l, "ref refs/heads/s1") {
			line = l
		}
	}
	if line == "" {
		t.Fatalf("no line names refs/heads/s1: %q", diff)
	}
	if !strings.Contains(line, oldSHA) || !strings.Contains(line, newSHA) {
		t.Fatalf("line %q does not show both values %s and %s", line, oldSHA, newSHA)
	}
}

// TestFingerprintMeetsTimingBudgetOn500Files is AC-4: the budget is 50 ms on a 500-file fixture.
func TestFingerprintMeetsTimingBudgetOn500Files(t *testing.T) {
	r := forge.New(t)
	for i := 0; i < 500; i++ {
		r.WriteFile(fmt.Sprintf("dir%02d/file%03d.txt", i%20, i), fmt.Sprintf("content %d\n", i))
	}
	r.Commit("500 files")

	var worst time.Duration
	for i := 0; i < 5; i++ {
		start := time.Now()
		takeT(t, r.Path)
		if d := time.Since(start); d > worst {
			worst = d
		}
	}
	t.Logf("500-file fixture: worst of 5 Take calls = %v (budget 50ms)", worst)
	if worst > 50*time.Millisecond {
		t.Fatalf("Take took %v on 500 files; budget 50ms", worst)
	}
}

// TestNoFalsePositivesAcrossTwentyEmptyRuns is AC-5, on a clean repository and on a dirty one.
func TestNoFalsePositivesAcrossTwentyEmptyRuns(t *testing.T) {
	for _, dirty := range []bool{false, true} {
		name := "clean"
		if dirty {
			name = "dirty"
		}
		t.Run(name, func(t *testing.T) {
			r := baseline(t)
			if dirty {
				r.WriteFile("shared.txt", "line1\nuncommitted\n")
				r.WriteFile("new.txt", "n\n")
			}
			first := takeT(t, r.Path)
			for i := 1; i <= 20; i++ {
				if d := Diff(first, takeT(t, r.Path)); len(d) != 0 {
					t.Fatalf("run %d of an empty operation reported %q", i, d)
				}
			}
		})
	}
}

// TestTakeRefusesANonWorkTree: a plain directory is not a repository, so Take must fail, not return
// an empty fingerprint.
func TestTakeRefusesANonWorkTree(t *testing.T) {
	if _, err := Take(t.TempDir()); err == nil {
		t.Fatal("Take on a plain directory succeeded")
	}
}

// TestAmbientGitDirCannotRedirectTheFingerprint: a hook or CI step may export GIT_DIR. The fingerprint
// must still describe the repository it was given.
func TestAmbientGitDirCannotRedirectTheFingerprint(t *testing.T) {
	r := baseline(t)
	other := forge.New(t)
	other.WriteFile("elsewhere.txt", "x\n")
	other.Commit("elsewhere")
	want := takeT(t, r.Path)

	t.Setenv("GIT_DIR", filepath.Join(other.Path, ".git"))
	t.Setenv("GIT_INDEX_FILE", filepath.Join(other.Path, ".git", "index"))

	if d := Diff(want, takeT(t, r.Path)); len(d) != 0 {
		t.Fatalf("ambient GIT_DIR redirected the fingerprint: %q", d)
	}
}

// must panics on error. Table closures run in subtests, so they must not call the parent's Fatal.
func must(err error) {
	if err != nil {
		panic(err)
	}
}
