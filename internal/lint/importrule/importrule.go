// Package importrule enforces the dependency matrix of architecture/01 §3 on
// production Go files. It parses imports only; it never compiles or runs the code it checks.
//
// Scope: non-test .go files under cmd/ and internal/. Test files and testdata
// directories are excluded, because the matrix governs production dependencies.
package importrule

import (
	"fmt"
	"go/parser"
	"go/token"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// ModulePath is the Go module path fixed by decision D-17.
const ModulePath = "github.com/Sumeet-basfore/ilvuno-code"

// Rule constrains the module-internal imports of one package directory.
// Packages without a rule are unconstrained except by the global rules.
type Rule struct {
	// Package is the directory relative to the module root, for example "internal/engine".
	Package string
	// Allowed lists the only module-internal packages this package may import
	// (relative paths). An empty list means no module-internal imports at all.
	Allowed []string
}

// Matrix is the M0 dependency matrix. Later milestones add rules with their packages.
var Matrix = []Rule{
	{Package: "cmd/ilvuno", Allowed: nil},
	{Package: "internal/engine", Allowed: []string{
		"internal/gitx", "internal/analyze", "internal/verify",
		"internal/evidence", "internal/model", "internal/policy", "internal/proc",
	}},
	{Package: "internal/model", Allowed: nil},
	{Package: "internal/policy", Allowed: nil},
}

// Violation describes one forbidden import.
type Violation struct {
	File   string // path relative to the checked root
	Import string
	Reason string
}

func (v Violation) String() string {
	return fmt.Sprintf("%s: imports %q: %s", v.File, v.Import, v.Reason)
}

// Check parses every production .go file under cmd/ and internal/ below root and
// returns the violations of the global rules and of Matrix, sorted by file and import.
func Check(root string) ([]Violation, error) {
	rules := make(map[string]Rule, len(Matrix))
	for _, r := range Matrix {
		rules[r.Package] = r
	}
	var out []Violation
	fset := token.NewFileSet()
	for _, top := range []string{"cmd", "internal"} {
		start := filepath.Join(root, top)
		if _, err := os.Stat(start); os.IsNotExist(err) {
			continue
		}
		err := filepath.WalkDir(start, func(path string, d fs.DirEntry, walkErr error) error {
			if walkErr != nil {
				return fmt.Errorf("walk %s: %w", path, walkErr)
			}
			if d.IsDir() {
				if d.Name() == "testdata" || strings.HasPrefix(d.Name(), ".") {
					return filepath.SkipDir
				}
				return nil
			}
			if !strings.HasSuffix(path, ".go") || strings.HasSuffix(path, "_test.go") {
				return nil
			}
			rel, err := filepath.Rel(root, path)
			if err != nil {
				return fmt.Errorf("relative path for %s: %w", path, err)
			}
			vs, err := checkFile(fset, path, filepath.ToSlash(rel), rules)
			if err != nil {
				return err
			}
			out = append(out, vs...)
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].File != out[j].File {
			return out[i].File < out[j].File
		}
		return out[i].Import < out[j].Import
	})
	return out, nil
}

func checkFile(fset *token.FileSet, path, rel string, rules map[string]Rule) ([]Violation, error) {
	f, err := parser.ParseFile(fset, path, nil, parser.ImportsOnly)
	if err != nil {
		return nil, fmt.Errorf("parse imports of %s: %w", rel, err)
	}
	pkgDir := filepath.ToSlash(filepath.Dir(rel))
	rule, hasRule := rules[pkgDir]
	var out []Violation
	for _, spec := range f.Imports {
		p, err := strconv.Unquote(spec.Path.Value)
		if err != nil {
			return nil, fmt.Errorf("unquote import in %s: %w", rel, err)
		}
		if v, bad := globalViolation(rel, p); bad {
			out = append(out, v)
			continue
		}
		if !strings.HasPrefix(p, ModulePath+"/") || !hasRule {
			continue
		}
		target := strings.TrimPrefix(p, ModulePath+"/")
		if !contains(rule.Allowed, target) {
			out = append(out, Violation{
				File:   rel,
				Import: p,
				Reason: fmt.Sprintf("package %s may not import %s", pkgDir, target),
			})
		}
	}
	return out, nil
}

// globalViolation applies the rules that hold for every package: no third-party
// modules, because the module has zero dependencies in M0.
func globalViolation(rel, importPath string) (Violation, bool) {
	if strings.HasPrefix(importPath, ModulePath+"/") || importPath == ModulePath {
		return Violation{}, false
	}
	first, _, _ := strings.Cut(importPath, "/")
	if strings.Contains(first, ".") {
		return Violation{File: rel, Import: importPath, Reason: "third-party module (zero dependencies in M0)"}, true
	}
	return Violation{}, false
}

func contains(list []string, s string) bool {
	for _, x := range list {
		if x == s {
			return true
		}
	}
	return false
}
