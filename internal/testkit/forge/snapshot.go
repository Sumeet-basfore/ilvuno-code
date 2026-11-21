package forge

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"sort"
	"strings"
)

// Snapshot is the state of a forge repository that equality assertions compare:
// every ref and its target, HEAD, the HEAD tree, and a digest of the index.
type Snapshot struct {
	Refs  map[string]string // full ref name -> object SHA
	Head  string            // commit SHA, or empty when HEAD is unborn
	Tree  string            // tree SHA of HEAD, or empty when unborn
	Index string            // SHA-256 of `ls-files --stage -z`
}

// Snapshot captures the repository state.
func (r *Repo) Snapshot() Snapshot {
	r.t.Helper()
	s := Snapshot{Refs: map[string]string{}}
	refs := r.MustGit("for-each-ref", "--format=%(refname) %(objectname)")
	for _, line := range strings.Split(refs, "\n") {
		if line == "" {
			continue
		}
		name, sha, ok := strings.Cut(line, " ")
		if !ok {
			r.t.Fatalf("forge: malformed for-each-ref line %q", line)
		}
		s.Refs[name] = sha
	}
	if head, err := r.Git("rev-parse", "--verify", "-q", "HEAD"); err == nil {
		s.Head = head
		s.Tree = r.MustGit("rev-parse", "HEAD^{tree}")
	}
	index := r.MustGit("ls-files", "--stage", "-z")
	sum := sha256.Sum256([]byte(index))
	s.Index = hex.EncodeToString(sum[:])
	return s
}

// Equal reports whether two snapshots describe identical repository state.
func (s Snapshot) Equal(o Snapshot) bool {
	return s.Diff(o) == ""
}

// Diff names every field that differs between two snapshots, or returns "" when they are equal.
func (s Snapshot) Diff(o Snapshot) string {
	var parts []string
	if s.Head != o.Head {
		parts = append(parts, fmt.Sprintf("head %q != %q", s.Head, o.Head))
	}
	if s.Tree != o.Tree {
		parts = append(parts, fmt.Sprintf("tree %q != %q", s.Tree, o.Tree))
	}
	if s.Index != o.Index {
		parts = append(parts, fmt.Sprintf("index digest %q != %q", s.Index, o.Index))
	}
	names := map[string]bool{}
	for k := range s.Refs {
		names[k] = true
	}
	for k := range o.Refs {
		names[k] = true
	}
	sorted := make([]string, 0, len(names))
	for k := range names {
		sorted = append(sorted, k)
	}
	sort.Strings(sorted)
	for _, k := range sorted {
		if s.Refs[k] != o.Refs[k] {
			parts = append(parts, fmt.Sprintf("ref %s %q != %q", k, s.Refs[k], o.Refs[k]))
		}
	}
	return strings.Join(parts, "; ")
}
