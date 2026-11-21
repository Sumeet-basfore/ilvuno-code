package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f09Cleanup is F9-cleanup: the cleanup-failure fault injection (SP-1R3 F9-cleanup). It has the shared-line
// edits of F1 and no extra files. It is a test case, not a corpus entry, so it is not in All(). With
// Options{FailCleanup: true} it expects ok with exit 5, and the orphaned worktree is reported.
var f09Cleanup = Fixture{
	ID:    "F9-cleanup",
	Class: "fault injection: cleanup failure (S-1); not a repository class",
	Build: buildF9,
}

func buildF9(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-line2"))
	s2 := branchCommit(r, "s2", base, "s2", setShared(r, 10, "s2-line10"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}
