package fixtures

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// f08DriverMarker is the path the F8 driver writes when git runs it. It lies in the fixture's temporary root,
// outside the repository, and the forge forwards it to git as DRIVER_MARKER. The driver command is unchanged.
func f08DriverMarker(r *forge.Repo) string { return filepath.Join(r.TempRoot(), "driver-marker") }

// driverCommand is the F8 merge driver, byte-identical to sp1.sh. The fixture only configures it. Nothing in
// this package runs a merge driver, and the harness refuses the repository before any merge.
const driverCommand = `sh -c 'echo ran >> "${DRIVER_MARKER:-/nonexistent}"; tr a-z A-Z < %A > %A.tmp && mv %A.tmp %A'`

// f08Driver is F8-driver: a custom merge driver is configured on a conflicting path (D-08). The driver must not
// run. Expected: harness_refused, exit 4.
var f08Driver = Fixture{
	ID:    "F8-driver",
	Class: "custom merge driver configured (D-08); driver must not run",
	Build: buildF8,
	Check: checkF8,
}

// buildF8 configures a custom merge driver on a conflicting path. The driver is never run here.
func buildF8(r *forge.Repo) Built {
	commonBase(r) // sp1.sh commits the common base first, then this case's own base
	r.WriteFile(".gitattributes", "driverfile.txt merge=upper\n")
	r.WriteFile("driverfile.txt", "hello world\n")
	base := r.Commit("base")
	r.MustGit("config", "merge.upper.driver", driverCommand)
	r.SetEnv("DRIVER_MARKER", f08DriverMarker(r))
	s1 := branchCommit(r, "s1", base, "s1", writeFile(r, "driverfile.txt", "hello from s1\n"))
	s2 := branchCommit(r, "s2", base, "s2", writeFile(r, "driverfile.txt", "hello from s2\n"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}

// checkF8 asserts the rest of the F8 contract. The driver is configured. The refusal came before any merge, so
// the candidate is empty, main is still at the base, and no worktree was added. The driver marker, which the
// driver writes when git runs it, does not exist. The control in f08_driver_test.go shows that the marker is
// observable: the same driver writes it when plain git merges, so its absence here is evidence that the driver
// did not run.
func checkF8(t testing.TB, r *forge.Repo, built Built, got Verdict) {
	t.Helper()
	if _, err := os.Stat(f08DriverMarker(r)); err == nil {
		t.Fatalf("F8: the driver marker %s exists after a refused run; the driver ran", f08DriverMarker(r))
	}
	if v, err := r.Git("config", "--local", "--get", "merge.upper.driver"); err != nil || !strings.Contains(v, "sh -c") {
		t.Fatalf("F8: merge.upper.driver is not configured as expected: %q, %v", v, err)
	}
	if got.Candidate != "" {
		t.Fatalf("F8: candidate %s was produced; a refused run must not merge", got.Candidate)
	}
	if head := r.Rev("main"); head != built.Base {
		t.Fatalf("F8: main moved to %s, base is %s: a merge ran", head, built.Base)
	}
	if n := strings.Count(r.MustGit("worktree", "list", "--porcelain"), "\nworktree ") + 1; n != 1 {
		t.Fatalf("F8: %d worktrees exist after a refused run, want 1 (the main worktree)", n)
	}
}
