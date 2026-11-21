package fixtures

import "github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"

// checkScript is the F7 check, byte-identical to build_fixture in sp1.sh.
const checkScript = "#!/bin/sh\n" +
	"n=$(cat config/*.txt 2>/dev/null | grep -c .)\n" +
	"[ \"$n\" -eq 1 ] || { echo \"expected 1 region, found $n\"; exit 1; }\n" +
	"exit 0\n"

// f07CombinedBuildFailure is F7-combined-build-failure: the check fails only on the combined candidate. Each
// source alone passes it (T-API-1 style). Expected: check_failed, exit 1.
var f07CombinedBuildFailure = Fixture{
	ID:    "F7-combined-build-failure",
	Class: "check fails only in combination (T-API-1 style)",
	Build: buildF7,
}

// buildF7 adds a check script and a config directory to the base. Each source alone passes the check; the
// combined candidate has two regions and fails it.
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
