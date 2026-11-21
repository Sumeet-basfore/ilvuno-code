package fixtures

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// referenceDir is the SP-1R3 reference copy (T-0005). The tests read the recorded expectations from it
// and never write to it.
const referenceDir = "../../../testdata/reference"

// expectedRow is one row of expected.json, keyed "<fixture>|<approach>|<variant>".
type expectedRow struct {
	Base          string   `json:"base"`
	Candidate     string   `json:"candidate"`
	Verdict       string   `json:"verdict"`
	Exit          int      `json:"exit"`
	ConflictStep  int      `json:"conflict_step"`
	ConflictPaths []string `json:"conflict_paths"`
	Cleanup       string   `json:"cleanup"`
}

type expectedFile struct {
	Cases map[string]expectedRow `json:"cases"`
}

// oracleFile is the part of oracle.json the tests use: the base and source SHAs of each fixture.
type oracleFile struct {
	Fixtures map[string]struct {
		Base    string   `json:"base"`
		Sources []string `json:"sources"`
	} `json:"fixtures"`
}

// loadJSON decodes one file of the reference copy into v.
func loadJSON(t testing.TB, name string, v any) {
	t.Helper()
	path := filepath.Join(referenceDir, name)
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	if err := json.Unmarshal(b, v); err != nil {
		t.Fatalf("decode %s: %v", path, err)
	}
}

// expectedRowFor returns the expected.json row with the given key.
func expectedRowFor(t testing.TB, key string) expectedRow {
	t.Helper()
	var e expectedFile
	loadJSON(t, "expected.json", &e)
	row, ok := e.Cases[key]
	if !ok {
		t.Fatalf("expected.json has no row %s", key)
	}
	return row
}

// oracleRecord returns the oracle.json record of one fixture.
func oracleRecord(t testing.TB, id string) (base string, sources []string) {
	t.Helper()
	var o oracleFile
	loadJSON(t, "oracle.json", &o)
	rec, ok := o.Fixtures[id]
	if !ok {
		t.Fatalf("oracle.json has no record for %s", id)
	}
	return rec.Base, rec.Sources
}

// compareWithExpected checks got against the expected.json row for key. A refused run has no conflict or
// candidate to compare. A row whose cleanup is "removed" must not report a cleanup failure.
func compareWithExpected(t testing.TB, key string, got Verdict) {
	t.Helper()
	row := expectedRowFor(t, key)
	if got.Status != row.Verdict {
		t.Fatalf("%s: verdict = %q, want %q", key, got.Status, row.Verdict)
	}
	if got.Exit != row.Exit {
		t.Fatalf("%s: exit = %d, want %d", key, got.Exit, row.Exit)
	}
	if got.Status == "harness_refused" {
		return
	}
	if got.ConflictStep != row.ConflictStep {
		t.Fatalf("%s: conflict_step = %d, want %d", key, got.ConflictStep, row.ConflictStep)
	}
	if !equalStrings(got.ConflictPaths, row.ConflictPaths) {
		t.Fatalf("%s: conflict_paths = %q, want %q", key, got.ConflictPaths, row.ConflictPaths)
	}
	if got.Candidate != row.Candidate {
		t.Fatalf("%s: candidate = %s, want %s", key, got.Candidate, row.Candidate)
	}
	if row.Cleanup == "removed" && got.CleanupFailed {
		t.Fatalf("%s: cleanup failed, but expected.json records it as removed", key)
	}
}

func equalStrings(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
