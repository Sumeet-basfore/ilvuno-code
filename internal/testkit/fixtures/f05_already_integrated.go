package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f05AlreadyIntegrated is F5-already-integrated: the first source is already an ancestor of the base, so the
// harness skips it without an empty merge commit. Expected: ok, exit 0 (skip).
var f05AlreadyIntegrated = Fixture{
	ID:    "F5-already-integrated",
	Class: "source already in base (T-SEQ-2)",
	Build: buildF5,
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
