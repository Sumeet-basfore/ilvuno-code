package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f01TwoClean is F1-two-clean: a clean sequence of two sources, each editing its own shared line and
// adding its own file. Expected: ok, exit 0.
var f01TwoClean = Fixture{
	ID:    "F1-two-clean",
	Class: "clean sequence, 2 sources",
	Build: buildF1,
}

func buildF1(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", both(setShared(r, 2, "s1-line2"), writeFile(r, "s1.txt", "one\n")))
	s2 := branchCommit(r, "s2", base, "s2", both(setShared(r, 10, "s2-line10"), writeFile(r, "s2.txt", "two\n")))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2}}
}
