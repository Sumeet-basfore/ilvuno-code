package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f04ConflictStep2 is F4-conflict-step2: the first source merges cleanly, and the second conflicts with the
// first at step 2. Source 3 is clean on its own, so the conflict is a sequence property, not a pair property.
// Expected: conflict at step 2, exit 3.
var f04ConflictStep2 = Fixture{
	ID:    "F4-conflict-step2",
	Class: "clean at step 1, conflict at step 2 (T-SEQ-1)",
	Build: buildF4,
}

func buildF4(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-wins"))
	s2 := branchCommit(r, "s2", base, "s2", setShared(r, 2, "s2-wins"))
	s3 := branchCommit(r, "s3", base, "s3", setShared(r, 11, "s3-line11"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2, s3}}
}
