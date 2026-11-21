package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f10NewlinePath is F10-newline-path: both sources edit a path that contains a newline. The conflicting path is
// reported byte-exact. Expected: conflict at step 2, exit 3.
var f10NewlinePath = Fixture{
	ID:    "F10-newline-path",
	Class: "conflict on a path containing a newline; byte-exact path and blob hash",
	Build: buildF10,
}

func buildF10(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", writeFile(r, "line\nbreak.txt", "v1\n"))
	s2 := branchCommit(r, "s2", base, "s2", writeFile(r, "line\nbreak.txt", "v2\n"))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}
