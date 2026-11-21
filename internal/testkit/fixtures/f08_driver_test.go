package fixtures

import (
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// TestT0003F8DriverMarkerIsObservable restores the F8 evidence the remediation requires. Two things must hold.
//
// First, the marker is observable: the driver writes ${DRIVER_MARKER} when git runs it. A control shows this by
// running the same conflicting merges with plain git, outside the harness, in the same repository.
//
// Second, the harness never creates it: the F8 run is refused before any merge, so the marker is absent after
// Assert. Both checks use the fixture's temporary root, not /nonexistent.
func TestT0003F8DriverMarkerIsObservable(t *testing.T) {
	r := forge.New(t)
	marker := f08DriverMarker(r)

	// The harness path: refused, so no merge runs and the marker is not created. Assert builds the fixture once.
	res := Assert(t, f08Driver, r)
	built := res.Built
	if res.Verdict.Status != "harness_refused" {
		t.Fatalf("F8 verdict %q, want harness_refused", res.Verdict.Status)
	}
	if _, err := os.Stat(marker); err == nil {
		t.Fatalf("the refused F8 run created the driver marker %s", marker)
	}

	// The control: plain git merges the same sources, so the driver runs and must create the marker. The marker path
	// is re-checked first, because a symlink there would redirect the driver's write outside the temporary root.
	if err := markerIsSafe(marker); err != nil {
		t.Fatal(err)
	}
	gitMergeStep(t, r, built)
	body, err := os.ReadFile(marker)
	if err != nil {
		t.Fatalf("control: the driver did not write its marker %s: %v", marker, err)
	}
	if !strings.Contains(string(body), "ran") {
		t.Fatalf("control: marker %s has content %q, want the driver's \"ran\"", marker, body)
	}
}

// markerIsSafe reports an error when the marker path is a symlink. A missing marker is safe, because the driver
// creates it. The marker lies directly in the temporary root, which is the only directory the forge creates it in.
// The check is made immediately before a merge runs, but it is not atomic with the write that follows.
func markerIsSafe(marker string) error {
	if info, err := os.Lstat(marker); err == nil && info.Mode()&os.ModeSymlink != 0 {
		return fmt.Errorf("the driver marker %s is a symlink; refusing to run the driver", marker)
	}
	return nil
}

// TestT0003F8MarkerSymlinkCreatedAfterSetEnvIsNotFollowed covers the gap that SetEnv cannot see: a symlink created
// after the variable is set. The guard must refuse, and the file outside the temporary root must not change.
func TestT0003F8MarkerSymlinkCreatedAfterSetEnvIsNotFollowed(t *testing.T) {
	r := forge.New(t)
	f08Driver.Build(r) // SetEnv runs here, while the marker path is still absent
	marker := f08DriverMarker(r)

	outDir := t.TempDir()
	victim := outDir + "/victim.txt"
	if err := os.WriteFile(victim, []byte("victim\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(victim, marker); err != nil {
		t.Fatal(err)
	}

	if err := markerIsSafe(marker); err == nil {
		t.Fatal("a symlinked marker created after SetEnv was not refused")
	}
	if body, err := os.ReadFile(victim); err != nil || string(body) != "victim\n" {
		t.Fatalf("the external victim changed: %q, %v", body, err)
	}
}
