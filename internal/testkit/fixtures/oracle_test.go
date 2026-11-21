package fixtures

import (
	"testing"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// TestT0002AC7ForgeBaseAndSourceShasMatchOracle is the cross-check that T-0002 AC-7 requires. For F1 to F10
// (F9 included), the base SHA the forge builds equals the base recorded in testdata/reference/oracle.json, and
// every source SHA matches the recorded source. A difference fails the test; none is documented, because none
// exists. The log lines are the evidence.
func TestT0002AC7ForgeBaseAndSourceShasMatchOracle(t *testing.T) {
	for _, fx := range append(All(), F9()) {
		fx := fx
		t.Run(fx.ID, func(t *testing.T) {
			r := forge.New(t)
			built := fx.Build(r)
			base, sources := oracleRecord(t, fx.ID)

			if built.Base != base {
				t.Fatalf("base = %s, oracle %s", built.Base, base)
			}
			if len(built.Sources) != len(sources) {
				t.Fatalf("%d sources, oracle %d", len(built.Sources), len(sources))
			}
			for i := range sources {
				if built.Sources[i] != sources[i] {
					t.Errorf("source %d = %s, oracle %s", i+1, built.Sources[i], sources[i])
				}
			}
			t.Logf("EVIDENCE | oracle | %s | base %s | %d sources match", fx.ID, base, len(sources))
		})
	}
}
