// Package fixtures ports the verified SP-1R3 failure corpus (T-0003, S-1) onto the forge.
//
// Each builder reproduces one case of docs/experiments/scripts/sp1.sh build_fixture, step for
// step, so the base and source commits have the same SHAs the reference oracle recorded
// (testdata/reference/oracle.json). Fixture IDs keep the SP-1R3 names so results cross-check.
//
// Ported: F1-F8 and F10. F9 (cleanup-failure injection) is a fault-injection test, not a
// repository fixture (see integrate_test.go). Other failure classes are deferred (S-1).
package fixtures

import (
	"fmt"
	"strings"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// Built is a fixture after construction.
type Built struct {
	Base    string   // base commit SHA (oracle.json "base")
	Sources []string // source commit SHAs, in integration order
}

// Fixture is one corpus entry.
type Fixture struct {
	ID    string // SP-1R3 fixture ID, for example "F1-two-clean"
	Build func(r *forge.Repo) Built
}

// checkScript is the F7 check, byte-identical to build_fixture in sp1.sh.
const checkScript = "#!/bin/sh\n" +
	"n=$(cat config/*.txt 2>/dev/null | grep -c .)\n" +
	"[ \"$n\" -eq 1 ] || { echo \"expected 1 region, found $n\"; exit 1; }\n" +
	"exit 0\n"

// driverCommand is the F8 merge driver, byte-identical to sp1.sh. The fixture only configures it;
// nothing in this package runs a merge driver.
const driverCommand = `sh -c 'echo ran >> "${DRIVER_MARKER:-/nonexistent}"; tr a-z A-Z < %A > %A.tmp && mv %A.tmp %A'`

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

// All returns the ported corpus in SP-1R3 order.
func All() []Fixture {
	return []Fixture{
		{"F1-two-clean", buildF1},
		{"F2-three-clean", buildF2},
		{"F3-direct-conflict", buildF3},
		{"F4-conflict-step2", buildF4},
		{"F5-already-integrated", buildF5},
		{"F6-unrelated", buildF6},
		{"F7-combined-build-failure", buildF7},
		{"F8-driver", buildF8},
		{"F10-newline-path", buildF10},
	}
}

// F9 is the cleanup-failure fault injection (SP-1R3 F9-cleanup): the same shared-line edits as
// F1 with no extra files. It is not part of All(), because it is a fault-injection test.
func F9() Fixture { return Fixture{"F9-cleanup", buildF9} }

func buildF9(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-line2"))
	s2 := branchCommit(r, "s2", base, "s2", setShared(r, 10, "s2-line10"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}

// ByID returns the fixture with the given SP-1R3 ID, and false when it is not in the ported corpus.
func ByID(id string) (Fixture, bool) {
	for _, f := range All() {
		if f.ID == id {
			return f, true
		}
	}
	return Fixture{}, false
}

// Edit helpers keep each case readable. They mirror the sed and printf calls of sp1.sh.
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

func buildF1(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", both(setShared(r, 2, "s1-line2"), writeFile(r, "s1.txt", "one\n")))
	s2 := branchCommit(r, "s2", base, "s2", both(setShared(r, 10, "s2-line10"), writeFile(r, "s2.txt", "two\n")))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}

func buildF2(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", both(setShared(r, 2, "s1-line2"), writeFile(r, "s1.txt", "one\n")))
	s2 := branchCommit(r, "s2", base, "s2", both(setShared(r, 10, "s2-line10"), writeFile(r, "s2.txt", "two\n")))
	s3 := branchCommit(r, "s3", base, "s3", both(setShared(r, 6, "s3-line6"), writeFile(r, "s3.txt", "three\n")))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2, s3}}
}

func buildF3(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-wins"))
	s2 := branchCommit(r, "s2", base, "s2", setShared(r, 2, "s2-wins"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}

func buildF4(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-wins"))
	s2 := branchCommit(r, "s2", base, "s2", setShared(r, 2, "s2-wins"))
	s3 := branchCommit(r, "s3", base, "s3", setShared(r, 11, "s3-line11"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2, s3}}
}

// buildF5 integrates s1 into main first, so s1 is already an ancestor of the new base.
func buildF5(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-line2"))
	r.Checkout("main")
	r.MustGit("merge", "-q", "--no-ff", "-m", "merge s1", s1)
	newBase := r.Rev("HEAD")
	s2 := branchCommit(r, "s2", newBase, "s2", setShared(r, 10, "s2-line10"))
	r.Checkout("main")
	return Built{Base: newBase, Sources: []string{s1, s2}}
}

// buildF6 adds a source with unrelated history (an orphan commit).
func buildF6(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-line2"))
	r.MustGit("checkout", "-q", "--orphan", "orphan")
	r.MustGit("rm", "-r", "-q", "-f", ".")
	r.WriteFile("u.txt", "unrelated\n")
	s2 := r.Commit("orphan")
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}

// buildF7 adds a check script and a config directory to the base. Each source alone passes the
// check; the combined candidate has two regions and fails it.
func buildF7(r *forge.Repo) Built {
	commonBase(r) // sp1.sh commits the common base first, then this case's own base
	r.WriteFile("config/.keep", "")
	r.WriteFile("check.sh", checkScript)
	r.Chmod("check.sh", 0o755)
	base := r.Commit("base")
	s1 := branchCommit(r, "s1", base, "s1", writeFile(r, "config/eu.txt", "eu\n"))
	s2 := branchCommit(r, "s2", base, "s2", writeFile(r, "config/us.txt", "us\n"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}

// buildF8 configures a custom merge driver on a conflicting path. The driver is never run here.
func buildF8(r *forge.Repo) Built {
	commonBase(r) // sp1.sh commits the common base first, then this case's own base
	r.WriteFile(".gitattributes", "driverfile.txt merge=upper\n")
	r.WriteFile("driverfile.txt", "hello world\n")
	base := r.Commit("base")
	r.MustGit("config", "merge.upper.driver", driverCommand)
	s1 := branchCommit(r, "s1", base, "s1", writeFile(r, "driverfile.txt", "hello from s1\n"))
	s2 := branchCommit(r, "s2", base, "s2", writeFile(r, "driverfile.txt", "hello from s2\n"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}

// buildF10 uses a path containing a newline, edited on both sides.
func buildF10(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", writeFile(r, "line\nbreak.txt", "v1\n"))
	s2 := branchCommit(r, "s2", base, "s2", writeFile(r, "line\nbreak.txt", "v2\n"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}
