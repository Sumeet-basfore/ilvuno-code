package fixtures

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// referenceDir is the SP-1R3 reference copy (T-0005). Tests read the recorded expectations from it.
const referenceDir = "../../../testdata/reference"

type expectedRow struct {
	Base          string   `json:"base"`
	Candidate     string   `json:"candidate"`
	Verdict       string   `json:"verdict"`
	Exit          int      `json:"exit"`
	ConflictStep  int      `json:"conflict_step"`
	ConflictPaths []string `json:"conflict_paths"`
	Cleanup       string   `json:"cleanup"`
}

type expectedFile struct {
	Cases map[string]expectedRow `json:"cases"`
}

type oracleFile struct {
	Fixtures map[string]struct {
		Base    string   `json:"base"`
		Sources []string `json:"sources"`
		Status  string   `json:"status"`
	} `json:"fixtures"`
}

func loadJSON(t *testing.T, name string, v any) {
	t.Helper()
	path := filepath.Join(referenceDir, name)
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s (T-0005 copies the reference here): %v", path, err)
	}
	if err := json.Unmarshal(b, v); err != nil {
		t.Fatalf("decode %s: %v", path, err)
	}
}

func loadExpected(t *testing.T) expectedFile {
	t.Helper()
	var e expectedFile
	loadJSON(t, "expected.json", &e)
	return e
}

func loadOracle(t *testing.T) oracleFile {
	t.Helper()
	var o oracleFile
	loadJSON(t, "oracle.json", &o)
	return o
}

// corpusWithF9 is the ported corpus plus the F9 fault-injection builder.
func corpusWithF9() []Fixture {
	return append(All(), F9())
}

func TestEveryPortedFixtureCarriesItsSPR3ID(t *testing.T) {
	// AC-1: every ported fixture has its SP-1R3 ID, and the reference recorded that ID.
	oracle := loadOracle(t)
	for _, fx := range corpusWithF9() {
		if _, ok := oracle.Fixtures[fx.ID]; !ok {
			t.Errorf("fixture %s has no oracle record", fx.ID)
		}
		if _, ok := ByID(fx.ID); fx.ID != "F9-cleanup" && !ok {
			t.Errorf("ByID(%s) not found", fx.ID)
		}
	}
}

func TestBaseAndSourceShasMatchOracle(t *testing.T) {
	// AC-7: forge-built base SHAs equal the oracle base, and every source SHA matches or is reported.
	oracle := loadOracle(t)
	for _, fx := range corpusWithF9() {
		fx := fx
		t.Run(fx.ID, func(t *testing.T) {
			r := forge.New(t)
			built := fx.Build(r)
			want := oracle.Fixtures[fx.ID]
			if built.Base != want.Base {
				t.Fatalf("base = %s, oracle %s", built.Base, want.Base)
			}
			if len(want.Sources) != 0 && len(want.Sources) != len(built.Sources) {
				t.Fatalf("source count %d, oracle %d", len(built.Sources), len(want.Sources))
			}
			for i := range want.Sources {
				if built.Sources[i] != want.Sources[i] {
					t.Errorf("source %d = %s, oracle %s (document this difference)", i+1, built.Sources[i], want.Sources[i])
				}
			}
		})
	}
}

func TestVerdictsAndCandidatesMatchExpectedJSON(t *testing.T) {
	// AC-2: verdict, exit, conflict step and paths, and candidate SHA match the recorded row.
	// The row is the object approach, plain variant.
	expected := loadExpected(t)
	for _, fx := range All() {
		fx := fx
		t.Run(fx.ID, func(t *testing.T) {
			row, ok := expected.Cases[fx.ID+"|object|plain"]
			if !ok {
				t.Fatalf("no expected row %s|object|plain", fx.ID)
			}
			r := forge.New(t)
			built := fx.Build(r)
			got, err := Integrate(r, built.Base, built.Sources, Options{})
			if err != nil {
				t.Fatalf("Integrate: %v", err)
			}
			if got.Status != row.Verdict {
				t.Fatalf("verdict = %q, want %q", got.Status, row.Verdict)
			}
			if got.Exit != row.Exit {
				t.Fatalf("exit = %d, want %d", got.Exit, row.Exit)
			}
			if got.Status == "harness_refused" {
				return
			}
			if got.ConflictStep != row.ConflictStep {
				t.Fatalf("conflict_step = %d, want %d", got.ConflictStep, row.ConflictStep)
			}
			if len(got.ConflictPaths) != len(row.ConflictPaths) {
				t.Fatalf("conflict_paths = %v, want %v", got.ConflictPaths, row.ConflictPaths)
			}
			for i := range row.ConflictPaths {
				if got.ConflictPaths[i] != row.ConflictPaths[i] {
					t.Fatalf("conflict_paths = %v, want %v", got.ConflictPaths, row.ConflictPaths)
				}
			}
			if got.Candidate != row.Candidate {
				t.Fatalf("candidate = %s, want %s", got.Candidate, row.Candidate)
			}
		})
	}
}

func TestEachSourceAloneIsGreenAndCombinedF7IsRed(t *testing.T) {
	// AC-4: F7 fails only on the combined candidate. Each source alone passes the check.
	fx, ok := ByID("F7-combined-build-failure")
	if !ok {
		t.Fatal("F7 missing from the corpus")
	}
	r := forge.New(t)
	built := fx.Build(r)
	s1, s2 := built.Sources[0], built.Sources[1]

	alone1, err := Integrate(r, built.Base, []string{s1}, Options{})
	if err != nil {
		t.Fatal(err)
	}
	alone2, err := Integrate(r, built.Base, []string{s2}, Options{})
	if err != nil {
		t.Fatal(err)
	}
	both, err := Integrate(r, built.Base, []string{s1, s2}, Options{})
	if err != nil {
		t.Fatal(err)
	}

	if alone1.Status != "ok" || alone2.Status != "ok" {
		t.Fatalf("single sources: %q and %q, want ok and ok", alone1.Status, alone2.Status)
	}
	if both.Status != "check_failed" || both.Exit != 1 {
		t.Fatalf("combined = %q exit %d, want check_failed exit 1", both.Status, both.Exit)
	}
}

func TestConflictingFixturesReportTheRealConflict(t *testing.T) {
	// AC-3: F3 and F4 each conflict at the stated step, and git's own merge also fails there.
	cases := []struct {
		id   string
		step int
	}{
		{"F3-direct-conflict", 2},
		{"F4-conflict-step2", 2},
	}
	for _, tc := range cases {
		tc := tc
		t.Run(tc.id, func(t *testing.T) {
			fx, ok := ByID(tc.id)
			if !ok {
				t.Fatalf("%s missing", tc.id)
			}
			r := forge.New(t)
			built := fx.Build(r)
			got, err := Integrate(r, built.Base, built.Sources, Options{})
			if err != nil {
				t.Fatal(err)
			}
			if got.Status != "conflict" || got.ConflictStep != tc.step {
				t.Fatalf("got %q at step %d, want conflict at step %d", got.Status, got.ConflictStep, tc.step)
			}
		})
	}
}

func TestF9FaultInjectionLeavesOrphanAndExitsFive(t *testing.T) {
	// F9: cleanup-failure injection. The verdict is ok with exit 5, and the orphaned worktree is reported.
	expected := loadExpected(t)
	row, ok := expected.Cases["F9-cleanup|object|fail-cleanup"]
	if !ok {
		t.Fatal("no expected row F9-cleanup|object|fail-cleanup")
	}
	f9 := F9()

	t.Run("injected failure", func(t *testing.T) {
		r := forge.New(t)
		built := f9.Build(r)
		got, err := Integrate(r, built.Base, built.Sources, Options{FailCleanup: true})
		if err != nil {
			t.Fatal(err)
		}
		if got.Status != row.Verdict || got.Exit != row.Exit {
			t.Fatalf("got %s exit %d, want %s exit %d", got.Status, got.Exit, row.Verdict, row.Exit)
		}
		if !got.CleanupFailed || got.WorktreeDir == "" {
			t.Fatalf("orphan not reported: %+v", got)
		}
		if _, err := os.Stat(got.WorktreeDir); err != nil {
			t.Fatalf("orphaned worktree %s missing: %v", got.WorktreeDir, err)
		}
		// Remove the injected orphan so the test leaves nothing behind.
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
		if got.Status != "ok" || got.Exit != 0 || got.CleanupFailed {
			t.Fatalf("clean run = %+v, want ok exit 0 with no cleanup failure", got)
		}
	})
}

func TestEachFixtureBuildsWithinFourHundredMilliseconds(t *testing.T) {
	// AC-5: every fixture builds in under 400 ms.
	for _, fx := range corpusWithF9() {
		fx := fx
		t.Run(fx.ID, func(t *testing.T) {
			start := time.Now()
			fx.Build(forge.New(t))
			if elapsed := time.Since(start); elapsed > 400*time.Millisecond {
				t.Fatalf("built in %v, budget 400ms", elapsed)
			}
		})
	}
}
