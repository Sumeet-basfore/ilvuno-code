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

// sample builds a small two-branch repository with a dirty-free main checkout.
func sample(t *testing.T) (*forge.Repo, Fingerprint) {
	t.Helper()
	r := forge.New(t)
	r.WriteFile("shared.txt", "line1\nline2\nline3\n")
	r.WriteFile("stable.txt", "base\n")
	r.Commit("base")
	r.Branch("s1", "main")
	r.SetLine("shared.txt", 2, "s1-line2")
	r.Commit("s1")
	r.Checkout("main")
	fp, err := Take(r.Path)
	if err != nil {
		t.Fatalf("Take: %v", err)
	}
	return r, fp
}

// mutations are the single changes AC-1 requires the fingerprint to detect.
var mutations = []struct {
	name   string
	apply  func(r *forge.Repo)
	field  string // the field the diff must name
	expect string // a substring the diff must contain
}{
	{"ref move", func(r *forge.Repo) { r.MustGit("update-ref", "refs/heads/s1", "main") }, "ref refs/heads/s1", "refs/heads/s1"},
	{"new ref", func(r *forge.Repo) { r.MustGit("update-ref", "refs/heads/extra", "main") }, "ref refs/heads/extra", "refs/heads/extra"},
	{"branch delete", func(r *forge.Repo) { r.MustGit("branch", "-D", "s1") }, "ref refs/heads/s1", "refs/heads/s1"},
	{"staged change", func(r *forge.Repo) {
		r.WriteFile("staged.txt", "s\n")
		r.MustGit("add", "staged.txt")
	}, "worktree", "worktree"},
	{"dirty file", func(r *forge.Repo) { r.WriteFile("stable.txt", "changed\n") }, "worktree", "worktree"},
	{"untracked file", func(r *forge.Repo) { r.WriteFile("untracked.txt", "u\n") }, "worktree", "worktree"},
	{"stash", func(r *forge.Repo) {
		r.WriteFile("stable.txt", "stashed\n")
		r.MustGit("stash", "push", "-q", "-m", "x")
	}, "stash", "stash"},
	{"config edit", func(r *forge.Repo) { r.MustGit("config", "user.signingkey", "not-a-real-key") }, "config", "config"},
}

func TestEachMutationIsDetectedAndNamed(t *testing.T) {
	for _, m := range mutations {
		m := m
		t.Run(m.name, func(t *testing.T) {
			// Arrange
			r, before := sample(t)

			// Act
			m.apply(r)
			after, err := Take(r.Path)
			if err != nil {
				t.Fatalf("Take after %s: %v", m.name, err)
			}

			// Assert: the change is detected, and the diff names the field with both values (AC-3).
			diff := Diff(before, after)
			if len(diff) == 0 {
				t.Fatalf("%s was not detected", m.name)
			}
			joined := strings.Join(diff, "; ")
			if !strings.Contains(joined, m.expect) {
				t.Fatalf("diff %q does not name %q", joined, m.expect)
			}
			if !strings.Contains(joined, "->") {
				t.Fatalf("diff %q does not show both values", joined)
			}
		})
	}
}

func TestCandidateNamespaceIsIgnored(t *testing.T) {
	// AC-2: refs under refs/ilvuno/ and the .ilvuno/ directory are Ilvuno's own, so they are not a change.
	r, before := sample(t)

	r.MustGit("update-ref", "refs/ilvuno/candidates/probe", "main")
	r.WriteFile(".ilvuno/worktrees/x", "candidate scratch\n")
	after, err := Take(r.Path)
	if err != nil {
		t.Fatal(err)
	}

	if diff := Diff(before, after); len(diff) != 0 {
		t.Fatalf("owned namespace reported as a change: %v", diff)
	}
}

func TestEmptyOperationHasNoFalsePositivesOverTwentyRuns(t *testing.T) {
	// AC-5: twenty consecutive fingerprints with no change are all equal.
	r, first := sample(t)
	for i := 1; i <= 20; i++ {
		fp, err := Take(r.Path)
		if err != nil {
			t.Fatalf("run %d: %v", i, err)
		}
		if diff := Diff(first, fp); len(diff) != 0 {
			t.Fatalf("run %d reported a change with no operation: %v", i, diff)
		}
	}
}

func TestFingerprintOfFiveHundredFilesIsFast(t *testing.T) {
	// AC-4: at most 50 ms on a 500-file repository. The best of five runs is reported, because
	// one run on a loaded machine is noise.
	r := forge.New(t)
	for i := 0; i < 500; i++ {
		r.WriteFile(filepath.Join("pkg", fmt.Sprintf("f%03d.txt", i)), fmt.Sprintf("file %d\n", i))
	}
	r.Commit("many")

	best := time.Duration(1<<63 - 1)
	for i := 0; i < 5; i++ {
		start := time.Now()
		if _, err := Take(r.Path); err != nil {
			t.Fatal(err)
		}
		if d := time.Since(start); d < best {
			best = d
		}
	}
	t.Logf("best of 5 fingerprints of 500 files: %v", best)
	if best > 50*time.Millisecond {
		t.Fatalf("best fingerprint %v exceeds 50ms", best)
	}
}

func TestTakeFailsOutsideAWorkTree(t *testing.T) {
	dir := t.TempDir()
	if _, err := Take(filepath.Join(dir, "missing")); err == nil {
		t.Fatal("Take on a missing directory succeeded")
	}
	// A plain directory is not a git work tree either.
	if err := os.WriteFile(filepath.Join(dir, "x"), []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := Take(dir); err == nil {
		t.Fatal("Take on a non-repository succeeded")
	}
}

func TestAnchoredNamespaceCheckRejectsLookalikeRef(t *testing.T) {
	// G0-5 (X-04 regression): a branch named refs/ilvuno/... lives under refs/heads/ and is the user's.
	// Only refs/ilvuno/ itself, anchored at the start of the name, is Ilvuno's namespace.
	r, before := sample(t)

	r.MustGit("update-ref", "refs/ilvuno/x", "main") // Ilvuno's own namespace: must be ignored
	ownedOnly, err := Take(r.Path)
	if err != nil {
		t.Fatal(err)
	}
	if diff := Diff(before, ownedOnly); len(diff) != 0 {
		t.Fatalf("refs/ilvuno/x reported as a user change: %v", diff)
	}

	r.MustGit("update-ref", "refs/heads/refs/ilvuno/x", "main") // lookalike under refs/heads/: must count
	withLookalike, err := Take(r.Path)
	if err != nil {
		t.Fatal(err)
	}
	diff := Diff(before, withLookalike)
	if !strings.Contains(strings.Join(diff, ";"), "ref refs/heads/refs/ilvuno/x") {
		t.Fatalf("refs/heads/refs/ilvuno/x was not counted as a user change: %v", diff)
	}
}
