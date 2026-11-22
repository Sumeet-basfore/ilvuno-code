// Package safety fingerprints the user-owned state of a repository (T-0004), so that a scenario can
// assert INV-1 mechanically: nothing the user owns changed during a run.
//
// Read-only by construction: every git child runs with --no-optional-locks, so git never refreshes or
// writes the index while the fingerprint is taken. Refs under refs/ilvuno/ and the .ilvuno/ directory
// belong to Ilvuno's own candidate namespace and are excluded, so candidate creation is not a false
// positive. Paths are read with -z and content with Lstat, so unusual names and symlinks are reported
// as themselves, not as deleted or as their targets.
package safety

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"sync"
)

const (
	ownedRefPrefix = "refs/ilvuno/"
	ownedDir       = ".ilvuno/"
	absent         = "<absent>"
	deleted        = "<deleted>"
)

// ambientGitVars redirect git to another repository or index. They are removed from the environment of
// every git child, so the fingerprint describes the directory it was given, whatever the caller exports.
var ambientGitVars = map[string]bool{
	"GIT_DIR":                          true,
	"GIT_WORK_TREE":                    true,
	"GIT_INDEX_FILE":                   true,
	"GIT_COMMON_DIR":                   true,
	"GIT_OBJECT_DIRECTORY":             true,
	"GIT_ALTERNATE_OBJECT_DIRECTORIES": true,
	"GIT_NAMESPACE":                    true,
}

// Fingerprint is a snapshot of the user-owned state of one repository.
type Fingerprint struct {
	Refs     map[string]string // refs outside refs/ilvuno/: name -> object SHA
	Head     string            // commit HEAD resolves to; "" when unborn
	HeadRef  string            // symbolic HEAD target, e.g. refs/heads/main; "" when detached
	Index    map[string]string // index entry -> "mode object"; unmerged entries are "path (stage N)"
	Worktree map[string]string // path with a status change or untracked -> "XY state"
	Stashes  []string          // stash commit SHAs, newest first
	Config   string            // SHA-256 of the repository's config file
}

// gitResult is the outcome of one read-only query: its stdout, or the error that ends the snapshot.
type gitResult struct {
	out string
	err error
}

// queries holds the outcome of each of the seven reads. Each field is written by exactly one goroutine
// and is read only after runConcurrently has returned.
type queries struct {
	refs, head, headRef, index, status, stash, config gitResult
}

// Take fingerprints the repository at dir. It fails when dir is not a git work tree; it never returns
// an empty fingerprint for a directory that is not a repository. The seven reads run concurrently, and
// their results are applied in a fixed order, so the fingerprint and the first error are the same as a
// serial read would give.
func Take(dir string) (Fingerprint, error) {
	fp := Fingerprint{
		Refs:     map[string]string{},
		Index:    map[string]string{},
		Worktree: map[string]string{},
	}
	err := runQueries(dir).apply(dir, &fp)
	return fp, err
}

// runQueries runs the seven reads concurrently. They are independent of one another; the config path
// lookup and the config read are sequenced inside their single goroutine.
func runQueries(dir string) queries {
	var q queries
	runConcurrently(
		func() { q.refs.out, q.refs.err = git(dir, "for-each-ref", "--format=%(refname) %(objectname)") },
		func() { q.head.out, q.head.err = gitOptional(dir, "rev-parse", "--verify", "-q", "HEAD") },
		func() { q.headRef.out, q.headRef.err = gitOptional(dir, "symbolic-ref", "-q", "HEAD") },
		func() { q.index.out, q.index.err = git(dir, "ls-files", "--stage", "-z") },
		func() {
			q.status.out, q.status.err = git(dir, "status", "--porcelain=v2", "-z", "--untracked-files=all")
		},
		func() { q.stash.out, q.stash.err = git(dir, "stash", "list", "--format=%H") },
		func() { q.config.out, q.config.err = configDigest(dir) },
	)
	return q
}

// apply writes the outcomes into fp in the order of the serial implementation. It returns the first
// failure in that order; fp then holds everything applied before it.
func (q queries) apply(dir string, fp *Fingerprint) error {
	if err := applyRefs(q.refs, fp); err != nil {
		return err
	}
	if err := applyHead(q.head, q.headRef, fp); err != nil {
		return err
	}
	if err := applyIndex(q.index, fp); err != nil {
		return err
	}
	if err := applyWorktree(dir, q.status, fp); err != nil {
		return err
	}
	if q.stash.err != nil {
		return q.stash.err
	}
	fp.Stashes = lines(q.stash.out)
	if q.config.err != nil {
		return q.config.err
	}
	fp.Config = q.config.out
	return nil
}

func applyRefs(res gitResult, fp *Fingerprint) error {
	if res.err != nil {
		return res.err
	}
	for _, line := range lines(res.out) {
		name, sha, ok := strings.Cut(line, " ")
		if !ok {
			return fmt.Errorf("malformed for-each-ref line %q", line)
		}
		if !strings.HasPrefix(name, ownedRefPrefix) {
			fp.Refs[name] = sha
		}
	}
	return nil
}

// applyHead records the commit and the symbolic target of HEAD. An unborn or detached HEAD is a valid
// state (git exits 1 for it) and leaves the field empty; any other failure is reported.
func applyHead(head, ref gitResult, fp *Fingerprint) error {
	if head.err != nil {
		return head.err
	}
	fp.Head = head.out
	if ref.err != nil {
		return ref.err
	}
	fp.HeadRef = ref.out
	return nil
}

func applyIndex(res gitResult, fp *Fingerprint) error {
	if res.err != nil {
		return res.err
	}
	for _, rec := range records(res.out) {
		meta, path, ok := strings.Cut(rec, "\t")
		if !ok {
			return fmt.Errorf("malformed ls-files record %q", rec)
		}
		if strings.HasPrefix(path, ownedDir) {
			continue
		}
		f := strings.Fields(meta) // mode, object, stage
		if len(f) != 3 {
			return fmt.Errorf("malformed ls-files metadata %q", meta)
		}
		key := path
		if f[2] != "0" {
			key = fmt.Sprintf("%s (stage %s)", path, f[2])
		}
		fp.Index[key] = f[0] + " " + f[1]
	}
	return nil
}

// applyWorktree records every path that status lists (changed, deleted, unmerged or untracked, with
// untracked files expanded) together with its current content state. The content states are read here,
// serially, once the status output is in hand.
func applyWorktree(dir string, res gitResult, fp *Fingerprint) error {
	if res.err != nil {
		return res.err
	}
	recs := records(res.out)
	for i := 0; i < len(recs); i++ {
		xy, path, renamed, ok := statusEntry(recs[i])
		if !ok {
			if strings.HasPrefix(recs[i], "# ") || strings.HasPrefix(recs[i], "! ") {
				continue // headers and ignored entries are not state we track
			}
			return fmt.Errorf("unrecognized status record %q", recs[i])
		}
		if renamed {
			i++ // a rename's original path is the next record and is not a separate entry
		}
		if strings.HasPrefix(path, ownedDir) {
			continue
		}
		state, err := contentState(dir, path)
		if err != nil {
			return err
		}
		fp.Worktree[path] = xy + " " + state
	}
	return nil
}

// statusEntry splits one porcelain v2 -z record into its XY code and path. renamed reports a rename
// record, whose original path follows as a separate record. ok is false for records of another kind.
func statusEntry(rec string) (xy, path string, renamed, ok bool) {
	switch {
	case strings.HasPrefix(rec, "1 "):
		f := strings.SplitN(rec, " ", 9)
		if len(f) == 9 {
			return f[1], f[8], false, true
		}
	case strings.HasPrefix(rec, "2 "):
		f := strings.SplitN(rec, " ", 10)
		if len(f) == 10 {
			return f[1], f[9], true, true
		}
	case strings.HasPrefix(rec, "u "):
		f := strings.SplitN(rec, " ", 11)
		if len(f) == 11 {
			return f[1], f[10], false, true
		}
	case strings.HasPrefix(rec, "? "):
		return "??", rec[2:], false, true
	}
	return "", "", false, false
}

// contentState describes one path without following symlinks: a symlink by its target, a regular file
// by its mode and SHA-256, a missing path as deleted.
func contentState(dir, rel string) (string, error) {
	p := filepath.Join(dir, rel)
	info, err := os.Lstat(p)
	switch {
	case errors.Is(err, fs.ErrNotExist):
		return deleted, nil
	case err != nil:
		return "", fmt.Errorf("stat %s: %w", rel, err)
	case info.Mode()&fs.ModeSymlink != 0:
		target, err := os.Readlink(p)
		if err != nil {
			return "", fmt.Errorf("readlink %s: %w", rel, err)
		}
		return "link " + target, nil
	case info.IsDir():
		return "dir", nil
	case info.Mode().IsRegular():
		b, err := os.ReadFile(p)
		if err != nil {
			return "", fmt.Errorf("read %s: %w", rel, err)
		}
		return fmt.Sprintf("%04o sha256:%s", info.Mode().Perm(), digest(b)), nil
	default:
		return "other " + info.Mode().String(), nil
	}
}

// configDigest digests the repository's config file. The file's path comes from git, so the lookup and
// the read stay in one goroutine, in that order.
func configDigest(dir string) (string, error) {
	p, err := git(dir, "rev-parse", "--git-path", "config")
	if err != nil {
		return "", err
	}
	if !filepath.IsAbs(p) {
		p = filepath.Join(dir, p)
	}
	b, err := os.ReadFile(p)
	if err != nil {
		return "", fmt.Errorf("read config: %w", err)
	}
	return digest(b), nil
}

// Diff names every field that differs between two fingerprints, with both values. It returns nil when
// the fingerprints are equal. Map fields are reported per key, so the output says which ref or path moved.
func Diff(a, b Fingerprint) []string {
	var out []string
	out = append(out, diffMap("ref", a.Refs, b.Refs)...)
	if a.Head != b.Head {
		out = append(out, fmt.Sprintf("HEAD: %s -> %s", orNone(a.Head), orNone(b.Head)))
	}
	if a.HeadRef != b.HeadRef {
		out = append(out, fmt.Sprintf("HEAD ref: %s -> %s", orNone(a.HeadRef), orNone(b.HeadRef)))
	}
	out = append(out, diffMap("index", a.Index, b.Index)...)
	out = append(out, diffMap("worktree", a.Worktree, b.Worktree)...)
	if !equalStrings(a.Stashes, b.Stashes) {
		out = append(out, fmt.Sprintf("stash: [%s] -> [%s]", strings.Join(a.Stashes, " "), strings.Join(b.Stashes, " ")))
	}
	if a.Config != b.Config {
		out = append(out, fmt.Sprintf("config: %s -> %s", a.Config, b.Config))
	}
	return out
}

// Equal reports whether two fingerprints are identical.
func Equal(a, b Fingerprint) bool { return len(Diff(a, b)) == 0 }

func diffMap(kind string, a, b map[string]string) []string {
	keys := make(map[string]bool, len(a)+len(b))
	for k := range a {
		keys[k] = true
	}
	for k := range b {
		keys[k] = true
	}
	sorted := make([]string, 0, len(keys))
	for k := range keys {
		sorted = append(sorted, k)
	}
	sort.Strings(sorted)

	var out []string
	for _, k := range sorted {
		av, aok := a[k]
		bv, bok := b[k]
		if aok == bok && av == bv {
			continue
		}
		out = append(out, fmt.Sprintf("%s %s: %s -> %s", kind, k, quoted(av, aok), quoted(bv, bok)))
	}
	return out
}

func quoted(v string, ok bool) string {
	if !ok {
		return absent
	}
	return fmt.Sprintf("%q", v)
}

func orNone(v string) string {
	if v == "" {
		return "<none>"
	}
	return v
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

// git runs one read-only git command in dir and returns its stdout without a trailing newline. Errors
// carry argv and stderr, and wrap the exit error so callers can inspect the status code.
func git(dir string, args ...string) (string, error) {
	full := append([]string{"--no-optional-locks", "-C", dir}, args...)
	cmd := exec.Command("git", full...)
	cmd.Env = gitEnv()
	var out, stderr bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return "", fmt.Errorf("git %s: %w: %s", strings.Join(args, " "), err, strings.TrimSpace(stderr.String()))
	}
	return strings.TrimRight(out.String(), "\n"), nil
}

// gitOptional is git for queries whose exit status 1 means "no value" (an unborn or detached HEAD).
// Any other failure is returned.
func gitOptional(dir string, args ...string) (string, error) {
	out, err := git(dir, args...)
	var exit *exec.ExitError
	if errors.As(err, &exit) && exit.ExitCode() == 1 {
		return "", nil
	}
	return out, err
}

func gitEnv() []string {
	env := make([]string, 0, len(os.Environ()))
	for _, kv := range os.Environ() {
		key, _, _ := strings.Cut(kv, "=")
		if !ambientGitVars[key] {
			env = append(env, kv)
		}
	}
	return env
}

// runConcurrently runs each job in its own goroutine and returns only after all of them have finished.
// The fan-out is the fixed number of jobs passed in, so no goroutine outlives the call.
func runConcurrently(jobs ...func()) {
	var wg sync.WaitGroup
	wg.Add(len(jobs))
	for _, job := range jobs {
		go func() {
			defer wg.Done()
			job()
		}()
	}
	wg.Wait()
}

// records splits NUL-terminated output (git -z). A trailing terminator yields no empty record.
func records(out string) []string {
	if out == "" {
		return nil
	}
	recs := strings.Split(out, "\x00")
	if recs[len(recs)-1] == "" {
		recs = recs[:len(recs)-1]
	}
	return recs
}

// lines splits newline-terminated output and drops empty lines.
func lines(out string) []string {
	var res []string
	for _, l := range strings.Split(out, "\n") {
		if l != "" {
			res = append(res, l)
		}
	}
	return res
}

func digest(b []byte) string {
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:])
}
