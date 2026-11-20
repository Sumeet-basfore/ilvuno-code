// Package safety fingerprints everything a user owns in a repository (T-0004), so that a scenario
// can assert INV-1 mechanically: the user's state is unchanged after a run.
//
// Read-only by construction: every git command runs with --no-optional-locks, so git never refreshes
// or writes the index while the fingerprint is taken. Refs under refs/ilvuno/ and the .ilvuno/ directory
// belong to Ilvuno's own candidate namespace and are excluded, so candidate creation is not a false positive.
package safety

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
)

// ownedRefPrefix is Ilvuno's candidate ref namespace (refs/ilvuno/* is excluded from the fingerprint).
const ownedRefPrefix = "refs/ilvuno/"

// ownedDir is Ilvuno's working directory inside the repository.
const ownedDir = ".ilvuno/"

// Fingerprint is a snapshot of the user-owned state of one repository.
type Fingerprint struct {
	Refs     map[string]string // refs outside refs/ilvuno/ -> object SHA
	Head     string            // HEAD commit SHA, or "" when unborn or detached without a commit
	Index    string            // SHA-256 of `ls-files --stage -z`
	Worktree string            // SHA-256 of the status output plus the content digest of each changed path
	Stashes  string            // SHA-256 of `stash list` SHAs, in order
	Config   string            // SHA-256 of the repository's config file
}

// Take fingerprints the repository at dir. It fails if dir is not a git work tree.
func Take(dir string) (Fingerprint, error) {
	fp := Fingerprint{Refs: map[string]string{}}

	refs, err := git(dir, "for-each-ref", "--format=%(refname) %(objectname)")
	if err != nil {
		return fp, err
	}
	for _, line := range strings.Split(refs, "\n") {
		if line == "" {
			continue
		}
		name, sha, ok := strings.Cut(line, " ")
		if !ok {
			return fp, fmt.Errorf("malformed for-each-ref line %q", line)
		}
		if strings.HasPrefix(name, ownedRefPrefix) {
			continue
		}
		fp.Refs[name] = sha
	}

	if head, err := git(dir, "rev-parse", "--verify", "-q", "HEAD"); err == nil {
		fp.Head = head
	}

	index, err := git(dir, "ls-files", "--stage", "-z")
	if err != nil {
		return fp, err
	}
	fp.Index = digest([]byte(index))

	wt, err := worktreeDigest(dir)
	if err != nil {
		return fp, err
	}
	fp.Worktree = wt

	stashes, err := git(dir, "stash", "list", "--format=%H")
	if err != nil {
		return fp, err
	}
	fp.Stashes = digest([]byte(stashes))

	cfgPath, err := git(dir, "rev-parse", "--git-path", "config")
	if err != nil {
		return fp, err
	}
	if !filepath.IsAbs(cfgPath) {
		cfgPath = filepath.Join(dir, cfgPath)
	}
	cfg, err := os.ReadFile(cfgPath)
	if err != nil {
		return fp, fmt.Errorf("read config: %w", err)
	}
	fp.Config = digest(cfg)
	return fp, nil
}

// worktreeDigest hashes the porcelain status (untracked files included, ownedDir excluded) together
// with the content of every listed path. A deleted path contributes a fixed marker.
func worktreeDigest(dir string) (string, error) {
	status, err := git(dir, "status", "--porcelain=v2", "--untracked-files=all")
	if err != nil {
		return "", err
	}
	var kept []string
	var paths []string
	sc := bufio.NewScanner(strings.NewReader(status))
	for sc.Scan() {
		line := sc.Text()
		path := statusPath(line)
		if path == "" || strings.HasPrefix(path, ownedDir) {
			continue
		}
		kept = append(kept, line)
		paths = append(paths, path)
	}
	h := sha256.New()
	h.Write([]byte(strings.Join(kept, "\n")))
	sort.Strings(paths)
	for _, p := range paths {
		h.Write([]byte("\x00" + p + "\x00"))
		b, err := os.ReadFile(filepath.Join(dir, p))
		switch {
		case err == nil:
			h.Write(b)
		case os.IsNotExist(err):
			h.Write([]byte("<deleted>"))
		default:
			return "", fmt.Errorf("read %s: %w", p, err)
		}
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}

// statusPath extracts the path from one porcelain v2 status line, or "" for header lines.
func statusPath(line string) string {
	switch {
	case strings.HasPrefix(line, "1 "):
		parts := strings.SplitN(line, " ", 9)
		if len(parts) == 9 {
			return parts[8]
		}
	case strings.HasPrefix(line, "2 "):
		// "2 XY ... <score> <path>\t<origPath>": the path is before the tab.
		head, _, _ := strings.Cut(line, "\t")
		parts := strings.SplitN(head, " ", 10)
		if len(parts) == 10 {
			return parts[9]
		}
	case strings.HasPrefix(line, "u "):
		parts := strings.SplitN(line, " ", 11)
		if len(parts) == 11 {
			return parts[10]
		}
	case strings.HasPrefix(line, "? "):
		return strings.TrimPrefix(line, "? ")
	}
	return ""
}

// git runs one read-only git command in dir and returns trimmed stdout.
func git(dir string, args ...string) (string, error) {
	full := append([]string{"--no-optional-locks", "-C", dir}, args...)
	cmd := exec.Command("git", full...)
	var out, stderr bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return "", fmt.Errorf("git %s: %w: %s", strings.Join(args, " "), err, strings.TrimSpace(stderr.String()))
	}
	return strings.TrimRight(out.String(), "\n"), nil
}

func digest(b []byte) string {
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:])
}

// Diff names every field that differs between two fingerprints, with both values.
// It returns nil when the fingerprints are equal.
func Diff(a, b Fingerprint) []string {
	var out []string
	names := map[string]bool{}
	for k := range a.Refs {
		names[k] = true
	}
	for k := range b.Refs {
		names[k] = true
	}
	sorted := make([]string, 0, len(names))
	for k := range names {
		sorted = append(sorted, k)
	}
	sort.Strings(sorted)
	for _, k := range sorted {
		if a.Refs[k] != b.Refs[k] {
			out = append(out, fmt.Sprintf("ref %s: %q -> %q", k, a.Refs[k], b.Refs[k]))
		}
	}
	scalar := []struct{ field, x, y string }{
		{"HEAD", a.Head, b.Head},
		{"index", a.Index, b.Index},
		{"worktree", a.Worktree, b.Worktree},
		{"stash", a.Stashes, b.Stashes},
		{"config", a.Config, b.Config},
	}
	for _, s := range scalar {
		if s.x != s.y {
			out = append(out, fmt.Sprintf("%s: %s -> %s", s.field, s.x, s.y))
		}
	}
	return out
}

// Equal reports whether two fingerprints are identical.
func Equal(a, b Fingerprint) bool { return len(Diff(a, b)) == 0 }
