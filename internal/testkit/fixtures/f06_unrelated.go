package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// f06Unrelated is F6-unrelated: the second source is an orphan commit with no common history. Expected:
// unrelated_histories at step 2, exit 3.
var f06Unrelated = Fixture{
	ID:    "F6-unrelated",
	Class: "unrelated histories (T-SEQ-3)",
	Build: buildF6,
}

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
