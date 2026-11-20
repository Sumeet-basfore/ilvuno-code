package importrule

import (
	"fmt"
	"sort"
	"testing"
)

// repoRoot is the module root relative to this package directory.
const repoRoot = "../../.."

func TestRepositoryObeysMatrix(t *testing.T) {
	violations, err := Check(repoRoot)
	if err != nil {
		t.Fatalf("Check(%q): %v", repoRoot, err)
	}
	for _, v := range violations {
		t.Errorf("violation: %s", v)
	}
}

func TestViolatingFixtureIsDetected(t *testing.T) {
	// Each case is one forbidden edge planted in testdata/violating.
	// Arrange: the fixture tree is parsed, never compiled (testdata is ignored by the go tool).
	violations, err := Check("testdata/violating")
	if err != nil {
		t.Fatalf("Check: %v", err)
	}
	got := make(map[string]bool, len(violations))
	for _, v := range violations {
		got[fmt.Sprintf("%s -> %s", v.File, v.Import)] = true
	}

	tests := []struct {
		name string
		edge string
	}{
		{"engine may not import agent", "internal/engine/engine.go -> " + ModulePath + "/internal/agent"},
		{"engine may not import a third-party module", "internal/engine/engine.go -> github.com/example/leftpad"},
		{"model is stdlib-only", "internal/model/model.go -> " + ModulePath + "/internal/policy"},
		{"cmd imports no internal package in M0", "cmd/ilvuno/main.go -> " + ModulePath + "/internal/model"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if !got[tc.edge] {
				t.Fatalf("expected violation %q, got %v", tc.edge, keys(got))
			}
		})
	}
}

func TestAllowedEdgeIsNotReported(t *testing.T) {
	violations, err := Check("testdata/violating")
	if err != nil {
		t.Fatalf("Check: %v", err)
	}
	for _, v := range violations {
		if v.File == "internal/engine/allowed.go" {
			t.Errorf("allowed edge reported: %s", v)
		}
	}
}

func TestViolatingFixtureHasExactlyTheExpectedCount(t *testing.T) {
	violations, err := Check("testdata/violating")
	if err != nil {
		t.Fatalf("Check: %v", err)
	}
	const want = 4
	if len(violations) != want {
		t.Fatalf("violations = %d, want %d: %v", len(violations), want, violations)
	}
}

func keys(m map[string]bool) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}
