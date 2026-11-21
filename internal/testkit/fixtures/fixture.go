// Package fixtures ports the verified SP-1R3 failure corpus (T-0003, S-1) onto the forge.
//
// Each fixture reproduces one case of docs/experiments/scripts/sp1.sh build_fixture step by step, so the
// base and source commits have the SHAs the reference oracle recorded (testdata/reference/oracle.json).
// Fixture IDs keep the SP-1R3 names, so results cross-check by name. One file per fixture: f01_* to f10_*.
//
// Deviation from the T-0003 outline: the outline's Spec() forge.Spec is Build(r *forge.Repo) Built. The
// forge is imperative (recorded in the T-0002 evidence), and a flat spec cannot express the orphan,
// merge and checkout steps that F5 and F6 need.
//
// Ported: F1 to F8 and F10. F9 (cleanup-failure injection) is a fault-injection case, not a repository
// fixture, and it is kept out of All(). The other failure classes are deferred (S-1).
package fixtures

import (
	"fmt"
	"strings"
	"testing"
	"time"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// Built is a fixture after construction.
type Built struct {
	Base    string   // base commit SHA (oracle.json "base")
	Sources []string // source commit SHAs, in integration order
}

// Fixture is one corpus entry.
type Fixture struct {
	ID    string                    // SP-1R3 fixture ID, for example "F1-two-clean"
	Class string                    // failure class, from the T-0003 scope table
	Build func(r *forge.Repo) Built // builds the fixture in r
	// Check is an optional extra assertion that runs after Assert has compared the verdict (F8).
	Check func(t testing.TB, r *forge.Repo, built Built, got Verdict)
}

// All returns the ported corpus in SP-1R3 order. F9 is not included; see F9.
func All() []Fixture {
	return []Fixture{
		f01TwoClean, f02ThreeClean, f03DirectConflict, f04ConflictStep2, f05AlreadyIntegrated,
		f06Unrelated, f07CombinedBuildFailure, f08Driver, f10NewlinePath,
	}
}

// F9 is the cleanup-failure fault injection (SP-1R3 F9-cleanup). It is a test case, not a corpus entry.
func F9() Fixture { return f09Cleanup }

// ByID returns the ported fixture with the given ID, and false when it is not in All().
func ByID(id string) (Fixture, bool) {
	for _, fx := range All() {
		if fx.ID == id {
			return fx, true
		}
	}
	return Fixture{}, false
}

// Result is one fixture after Assert: its commits, the verdict the harness reached, and the build time.
type Result struct {
	Built     Built
	Verdict   Verdict
	BuildTime time.Duration
}

// Assert builds fx in r, integrates it with the reference rules, and checks the outcome against the
// object/plain row of expected.json. The expected values are read from the reference copy and are not
// restated here. Assert then runs the fixture's Check, if it has one.
func Assert(t testing.TB, fx Fixture, r *forge.Repo) Result {
	t.Helper()
	start := time.Now()
	built := fx.Build(r)
	res := Result{Built: built, BuildTime: time.Since(start)}

	got, err := Integrate(r, built.Base, built.Sources, Options{})
	if err != nil {
		t.Fatalf("%s: Integrate: %v", fx.ID, err)
	}
	res.Verdict = got
	compareWithExpected(t, fx.ID+"|object|plain", got)
	if fx.Check != nil {
		fx.Check(t, r, built, got)
	}
	return res
}

// commonBase writes the shared base (.gitignore, shared.txt with 12 lines, stable.txt) and commits it.
func commonBase(r *forge.Repo) string {
	r.WriteFile(".gitignore", ".ilvuno/\n")
	var lines strings.Builder
	for i := 1; i <= 12; i++ {
		fmt.Fprintf(&lines, "line%d\n", i)
	}
	r.WriteFile("shared.txt", lines.String())
	r.WriteFile("stable.txt", "base\n")
	return r.Commit("base")
}

// branchCommit creates branch name at base, applies edit, and returns the commit SHA.
func branchCommit(r *forge.Repo, name, base, msg string, edit func()) string {
	r.Branch(name, base)
	edit()
	return r.Commit(msg)
}

// Edit helpers keep each builder readable. They mirror the sed and printf calls of sp1.sh.
func setShared(r *forge.Repo, line int, text string) func() {
	return func() { r.SetLine("shared.txt", line, text) }
}

func writeFile(r *forge.Repo, path, content string) func() {
	return func() { r.WriteFile(path, content) }
}

func both(fs ...func()) func() {
	return func() {
		for _, f := range fs {
			f()
		}
	}
}
