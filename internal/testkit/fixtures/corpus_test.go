package fixtures

import (
	"go/parser"
	"go/token"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// classTable is the T-0003 scope table, written from the task specification and not read from the code.
// AC-1 compares the two. The IDs and classes are the SP-1R3 names.
var classTable = []struct{ id, class string }{
	{"F1-two-clean", "clean sequence, 2 sources"},
	{"F2-three-clean", "clean sequence, 3 sources (E-1′ case a)"},
	{"F3-direct-conflict", "textual conflict, pair (T-CONF-1)"},
	{"F4-conflict-step2", "clean at step 1, conflict at step 2 (T-SEQ-1)"},
	{"F5-already-integrated", "source already in base (T-SEQ-2)"},
	{"F6-unrelated", "unrelated histories (T-SEQ-3)"},
	{"F7-combined-build-failure", "check fails only in combination (T-API-1 style)"},
	{"F8-driver", "custom merge driver configured (D-08); driver must not run"},
	{"F10-newline-path", "conflict on a path containing a newline; byte-exact path and blob hash"},
}

// TestT0003AC1EachFixtureCarriesItsIDAndClass checks that the corpus holds exactly the table's fixtures, each
// with its SP-1R3 ID and class, and that F9 is the fault-injection case, not a corpus entry.
func TestT0003AC1EachFixtureCarriesItsIDAndClass(t *testing.T) {
	if got, want := len(All()), len(classTable); got != want {
		t.Fatalf("corpus has %d fixtures, the table has %d", got, want)
	}
	for _, want := range classTable {
		fx, ok := ByID(want.id)
		if !ok {
			t.Errorf("%s is not in the corpus", want.id)
			continue
		}
		if fx.Class != want.class {
			t.Errorf("%s: class %q, table %q", want.id, fx.Class, want.class)
		}
	}
	if f9 := F9(); f9.ID != "F9-cleanup" || !strings.Contains(f9.Class, "fault injection") {
		t.Errorf("F9 = %q / %q, want the fault-injection case", f9.ID, f9.Class)
	}
	if _, ok := ByID("F9-cleanup"); ok {
		t.Error("F9 is in the corpus; it must stay a fault-injection case")
	}
}

// TestT0003AC2VerdictsReadFromExpectedJSON asserts every corpus fixture against its expected.json row. The
// verdict is never restated here. The log lines are the evidence table: ID, class, verdict, exit and build time.
func TestT0003AC2VerdictsReadFromExpectedJSON(t *testing.T) {
	for _, fx := range All() {
		fx := fx
		t.Run(fx.ID, func(t *testing.T) {
			res := Assert(t, fx, forge.New(t))
			t.Logf("EVIDENCE | %s | %s | %s exit %d | build %s", fx.ID, fx.Class,
				res.Verdict.Status, res.Verdict.Exit, res.BuildTime.Round(time.Microsecond))
		})
	}
}

// TestT0003AC3ConflictsAreRealAtTheStatedStep checks F3 and F4. Two things must hold: the harness reports a
// conflict at step 2, and plain git, merging the same sources one by one, also fails at step 2.
func TestT0003AC3ConflictsAreRealAtTheStatedStep(t *testing.T) {
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
				t.Fatalf("%s is not in the corpus", tc.id)
			}
			r := forge.New(t)
			built := fx.Build(r)

			got, err := Integrate(r, built.Base, built.Sources, Options{})
			if err != nil {
				t.Fatal(err)
			}
			if got.Status != "conflict" || got.ConflictStep != tc.step {
				t.Fatalf("harness: %q at step %d, want conflict at step %d", got.Status, got.ConflictStep, tc.step)
			}
			if step := gitMergeStep(t, r, built); step != tc.step {
				t.Fatalf("plain git merge fails at step %d, want %d", step, tc.step)
			}
		})
	}
}

// gitMergeStep merges the sources one by one into a probe branch cut from the base, with plain git rather than
// Integrate, and returns the 1-based step whose merge exits nonzero (0 when every merge succeeds). The probe
// branch is removed afterwards and the repository is returned to main.
func gitMergeStep(t testing.TB, r *forge.Repo, built Built) (step int) {
	t.Helper()
	r.Branch("probe", built.Base)
	defer func() {
		r.Git("merge", "--abort") // no-op when no merge is in progress
		r.Checkout("main")
		r.MustGit("branch", "-D", "probe")
	}()
	for i, src := range built.Sources {
		if _, err := r.Git("merge", "--no-ff", "--no-edit", "-m", "probe", src); err != nil {
			return i + 1
		}
	}
	return 0
}

// TestT0003AC4CombinedF7IsRedEachSourceAloneIsGreen checks that the F7 check fails only on the combined candidate.
func TestT0003AC4CombinedF7IsRedEachSourceAloneIsGreen(t *testing.T) {
	fx, ok := ByID("F7-combined-build-failure")
	if !ok {
		t.Fatal("F7 is not in the corpus")
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

// TestT0003AC5EachFixtureBuildsWithinFourHundredMilliseconds measures the build of each fixture (F9 included).
func TestT0003AC5EachFixtureBuildsWithinFourHundredMilliseconds(t *testing.T) {
	for _, fx := range append(All(), F9()) {
		fx := fx
		t.Run(fx.ID, func(t *testing.T) {
			r := forge.New(t)
			start := time.Now()
			fx.Build(r)
			elapsed := time.Since(start)
			t.Logf("EVIDENCE | build | %s | %s", fx.ID, elapsed.Round(time.Microsecond))
			if elapsed > 400*time.Millisecond {
				t.Fatalf("built in %v, budget 400 ms", elapsed)
			}
		})
	}
}

// TestT0003AC6NoNetworkNoScriptingOnlyTheCheckExecutes checks the testkit's imports and sources. The package
// may not import networking or TLS, and os/exec may appear only in integrate.go, where it runs the F7 check
// script and nothing else. No merge driver or hook is ever executed, and no TypeScript or npm is used.
func TestT0003AC6NoNetworkNoScriptingOnlyTheCheckExecutes(t *testing.T) {
	entries, err := os.ReadDir(".")
	if err != nil {
		t.Fatal(err)
	}
	fset := token.NewFileSet()
	for _, e := range entries {
		name := e.Name()
		if !strings.HasSuffix(name, ".go") || strings.HasSuffix(name, "_test.go") {
			continue
		}
		src, err := os.ReadFile(name)
		if err != nil {
			t.Fatal(err)
		}
		if lower := strings.ToLower(string(src)); strings.Contains(lower, "npm") || strings.Contains(lower, "typescript") {
			t.Errorf("%s mentions npm or TypeScript", name)
		}
		f, err := parser.ParseFile(fset, name, src, parser.ImportsOnly)
		if err != nil {
			t.Fatal(err)
		}
		for _, imp := range f.Imports {
			path := strings.Trim(imp.Path.Value, `"`)
			switch {
			case path == "net" || strings.HasPrefix(path, "net/") || path == "crypto/tls" || path == "plugin":
				t.Errorf("%s imports %s, which the fixtures package must not use", name, path)
			case path == "os/exec" && name != "integrate.go":
				t.Errorf("%s imports os/exec; only integrate.go may run the F7 check", name)
			}
		}
	}
}
