package fixtures

import (
	"os"
	"strings"
	"testing"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// TestT0003AC7F9FaultInjectionFailsTheCleanupStep applies the cleanup-failure injection to F9 and checks that
// the cleanup step fails: exit 5, the orphan is reported and exists, and the rest of the run is the recorded
// one. It then removes the orphan it created, so nothing is left behind. A run without injection must clean up.
func TestT0003AC7F9FaultInjectionFailsTheCleanupStep(t *testing.T) {
	f9 := F9()

	t.Run("injected failure", func(t *testing.T) {
		r := forge.New(t)
		built := f9.Build(r)
		got, err := Integrate(r, built.Base, built.Sources, Options{FailCleanup: true})
		if err != nil {
			t.Fatal(err)
		}
		compareWithExpected(t, "F9-cleanup|object|fail-cleanup", got)
		if !got.CleanupFailed || got.WorktreeDir == "" {
			t.Fatalf("orphan not reported: %+v", got)
		}
		if _, err := os.Stat(got.WorktreeDir); err != nil {
			t.Fatalf("orphaned worktree %s is missing: %v", got.WorktreeDir, err)
		}
		if err := os.RemoveAll(got.WorktreeDir); err != nil {
			t.Fatal(err)
		}
		r.MustGit("worktree", "prune")
	})

	t.Run("without injection", func(t *testing.T) {
		r := forge.New(t)
		built := f9.Build(r)
		got, err := Integrate(r, built.Base, built.Sources, Options{})
		if err != nil {
			t.Fatal(err)
		}
		compareWithExpected(t, "F9-cleanup|object|plain", got)
		if got.CleanupFailed {
			t.Fatalf("clean run reported a cleanup failure: %+v", got)
		}
		if n := strings.Count(r.MustGit("worktree", "list", "--porcelain"), "\nworktree ") + 1; n != 1 {
			t.Fatalf("%d worktrees after a clean run, want 1", n)
		}
	})
}
