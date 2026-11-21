package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f02ThreeClean is F2-three-clean: a clean sequence of three sources (E-1′ case a). Expected: ok, exit 0.
var f02ThreeClean = Fixture{
	ID:    "F2-three-clean",
	Class: "clean sequence, 3 sources (E-1′ case a)",
	Build: buildF2,
}

func buildF2(r *forge.Repo) Built {
	base := commonBase(r)
	s1 := branchCommit(r, "s1", base, "s1", both(setShared(r, 2, "s1-line2"), writeFile(r, "s1.txt", "one\n")))
	s2 := branchCommit(r, "s2", base, "s2", both(setShared(r, 10, "s2-line10"), writeFile(r, "s2.txt", "two\n")))
	s3 := branchCommit(r, "s3", base, "s3", both(setShared(r, 6, "s3-line6"), writeFile(r, "s3.txt", "three\n")))
	r.Checkout("main")
	return Built{Base: base, Sources: []string{s1, s2, s3}}
}
