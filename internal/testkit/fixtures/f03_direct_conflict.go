package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f03DirectConflict is F3-direct-conflict: two sources edit the same line of the same file. The conflict is
// at step 2, the first merge that meets the other source's edit. Expected: conflict, exit 3.
var f03DirectConflict = Fixture{
	ID:    "F3-direct-conflict",
	Class: "textual conflict, pair (T-CONF-1)",
	Build: buildF3,
}

func buildF3(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", setShared(r, 2, "s1-wins"))
	s2 := branchCommit(r, "s2", base, "s2", setShared(r, 2, "s2-wins"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}
