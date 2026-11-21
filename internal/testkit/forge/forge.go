// Package forge builds deterministic git repositories for tests (T-0002).
//
// Every repository uses the base-commit identity and date of the SP-1R3 reference harness
// (the sp1_env defaults in its lib.sh; see docs/experiments/scripts/README.md). The forge never
// reads the developer's global or system git configuration, never reads $HOME, and never
// touches the network (GIT_ALLOW_PROTOCOL=none).
//
// Deviation from the T-0002 outline: the outline proposed a declarative Spec. M0 fixtures
// need steps a flat spec expresses poorly (merge into main, orphan branch, checkout by
// name), so the forge exposes an imperative Repo API. Recorded in the T-0002 evidence.
package forge

import (
	"bytes"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// Reference identity and date for fixture commits: the sp1_env defaults the SP-1R3 harness uses to
// build base commits (fixture <fixture@example.com>, 2020-01-01T00:00:00Z). Base SHAs built with
// these values are what the oracle records (T-0002 AC-7). The integration envelope
// (ilvuno <ilvuno@localhost>, H-7) applies to merge commits made by the code under test, not to
// fixture bases, so the forge does not use it: using it would change every base SHA.
// One date serves every commit, as in the harness, so the oracle SHAs depend on no per-commit sequence.
const (
	Identity = "fixture"
	Email    = "fixture@example.com"
	Date     = "2020-01-01T00:00:00Z"
	// DateUnix is Date as seconds since the epoch, for exact %ct assertions.
	DateUnix = 1577836800
)

// Repo is a git repository created under t.TempDir().
//
// Containment: every path the forge writes, reads, removes or chmods is confined to the repository (Path), and
// every worktree it creates or removes is a direct child of TempRoot()/worktrees. The checks are made before each
// operation. They are not a sandbox: see resolve for the time-of-check window that remains.
type Repo struct {
	t     testing.TB
	Path  string
	home  string
	tmp   string
	hooks string
	extra []string // variables set with SetEnv, forwarded to every git child
}

// settableEnv lists the variables a fixture may set with SetEnv. It is deliberately small: the forge's own
// isolation variables (identity, configuration, protocol, locale, home) cannot be overridden, and GIT_* is
// refused. DRIVER_MARKER is the only one the fixtures use; it is read by the F8 merge driver.
var settableEnv = map[string]bool{"DRIVER_MARKER": true}

// SetEnv forwards one variable to every git child of this repository, so that a configured program can see it.
// Only the variables in settableEnv are accepted; anything else refuses the test. The value must be a clean
// absolute path strictly inside TempRoot: no traversal, and no symlink among its components. The check is made when
// the variable is set. A symlink created later is not seen here, so a fixture that writes the marker re-checks the
// path first (f08_driver_test.go).
func (r *Repo) SetEnv(key, value string) {
	r.t.Helper()
	if !settableEnv[key] {
		r.refuse("variable %q may not be set by a fixture", key)
	}
	if !filepath.IsAbs(value) || filepath.Clean(value) != value {
		r.refuse("value %q for %s is not a clean absolute path", value, key)
	}
	rel, err := filepath.Rel(r.TempRoot(), value)
	if err != nil || rel == "." {
		r.refuse("value %q for %s is not inside the temporary root %s", value, key, r.TempRoot())
	}
	if _, err := containedPath(r.TempRoot(), rel, "the temporary root"); err != nil {
		r.t.Fatalf("%v", err)
	}
	r.extra = append(r.extra, key+"="+value)
}

// New creates an empty repository on branch main. Each call creates a new, separate repository.
func New(t testing.TB) *Repo {
	t.Helper()
	base := t.TempDir()
	r := &Repo{
		t:     t,
		Path:  filepath.Join(base, "repo"),
		home:  filepath.Join(base, "home"),
		tmp:   filepath.Join(base, "tmp"),
		hooks: filepath.Join(base, "hooks"),
	}
	for _, d := range []string{r.Path, r.home, r.tmp, r.hooks} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatalf("forge: create %s: %v", d, err)
		}
	}
	if _, err := r.Git("init", "-q", "-b", "main", "."); err != nil {
		t.Fatalf("forge: %v", err)
	}
	return r
}

// TempRoot is the temporary directory that holds the repository, its worktrees and the isolated home. Nothing the
// forge creates lies outside it.
func (r *Repo) TempRoot() string { return filepath.Dir(r.Path) }

// environ is the complete environment for git children. It does not inherit the caller's environment,
// except PATH, so caller configuration variables cannot leak in.
func (r *Repo) environ() []string {
	return append([]string{
		"PATH=" + os.Getenv("PATH"),
		"HOME=" + r.home,
		"TMPDIR=" + r.tmp,
		"LANG=C",
		"LC_ALL=C",
		"TZ=UTC",
		"GIT_CONFIG_NOSYSTEM=1",
		"GIT_CONFIG_GLOBAL=/dev/null",
		"GIT_CONFIG_SYSTEM=/dev/null",
		"GIT_CONFIG_COUNT=0",
		"GIT_ATTR_NOSYSTEM=1",
		"GIT_ALLOW_PROTOCOL=none",
		"GIT_TERMINAL_PROMPT=0",
		"GIT_ASKPASS=/bin/false",
		"GIT_PAGER=cat",
		"GIT_EDITOR=true",
		"GIT_MERGE_AUTOEDIT=no",
		"GIT_AUTHOR_NAME=" + Identity,
		"GIT_AUTHOR_EMAIL=" + Email,
		"GIT_COMMITTER_NAME=" + Identity,
		"GIT_COMMITTER_EMAIL=" + Email,
		"GIT_AUTHOR_DATE=" + Date,
		"GIT_COMMITTER_DATE=" + Date,
	}, r.extra...)
}

// Git runs one git command in the repository and returns its stdout without the trailing newline.
// Errors name the argv and carry stderr, so failures are never silent.
func (r *Repo) Git(args ...string) (string, error) {
	return r.GitIn(r.Path, nil, args...)
}

// GitIn runs one git command in dir with extra environment entries appended to the fixed
// environment. Later entries override earlier ones, so callers can set commit identity and dates.
func (r *Repo) GitIn(dir string, extraEnv []string, args ...string) (string, error) {
	full := []string{
		"-c", "core.hooksPath=" + r.hooks,
		"-c", "core.attributesFile=/dev/null",
		"-c", "core.fsmonitor=false",
		"-c", "core.quotepath=false",
		"-c", "core.autocrlf=false",
		"-c", "core.eol=lf",
		"-c", "gc.auto=0",
		"-c", "commit.gpgsign=false",
		"-c", "tag.gpgsign=false",
		"-c", "submodule.recurse=false",
		"-c", "advice.detachedHead=false",
	}
	full = append(full, args...)
	cmd := exec.Command("git", full...)
	cmd.Dir = dir
	cmd.Env = append(r.environ(), extraEnv...)
	var out, stderr bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return "", fmt.Errorf("git %s: %w: %s", strings.Join(args, " "), err, strings.TrimSpace(stderr.String()))
	}
	return strings.TrimRight(out.String(), "\n"), nil
}

// MustGit is Git, failing the test on error.
func (r *Repo) MustGit(args ...string) string {
	r.t.Helper()
	out, err := r.Git(args...)
	if err != nil {
		r.t.Fatalf("forge: %v", err)
	}
	return out
}

// resolve returns the absolute path of a repository-relative path, after checking that the path stays inside the
// repository. It refuses absolute and traversing paths, and any existing component that is a symlink, the final
// component included. Each component is checked with Lstat, so a link is never followed during the check.
//
// Time-of-check window: the check and the operation that follows it are separate system calls. A process that
// replaces a checked component with a symlink in between can still redirect the operation. The forge's threat model
// is one test process at a time, not a concurrent same-UID adversary, and this is not a sandbox.
func (r *Repo) resolve(rel string) string {
	r.t.Helper()
	p, err := containedPath(r.Path, rel, "the repository")
	if err != nil {
		r.t.Fatalf("%v", err)
	}
	return p
}

// containedPath returns root/rel after checking that rel is a plain relative path and that no existing component
// below root is a symlink, the final component included. Its errors start with refusedPrefix when the refusal is
// a containment decision, and with "forge:" when the check itself failed. It is shared by resolve (the repository
// root) and SetEnv (the temporary root).
func containedPath(root, rel, what string) (string, error) {
	if !filepath.IsLocal(rel) {
		return "", errRefused("path %q escapes %s", rel, what)
	}
	cur := root
	for _, part := range strings.Split(filepath.Clean(rel), string(filepath.Separator)) {
		cur = filepath.Join(cur, part)
		info, err := os.Lstat(cur)
		if errors.Is(err, fs.ErrNotExist) {
			break // the remaining components do not exist either, so none can be a link
		}
		if err != nil {
			return "", fmt.Errorf("forge: lstat %s: %w", cur, err)
		}
		if info.Mode()&os.ModeSymlink != 0 {
			return "", errRefused("path %q has a symlink component %q", rel, part)
		}
	}
	return filepath.Join(root, rel), nil // the full path, not the prefix the walk stopped at
}

// refuse stops the test with a containment refusal. Every refusal message starts with refusedPrefix.
func (r *Repo) refuse(format string, args ...any) {
	r.t.Helper()
	r.t.Fatalf("forge: refused: "+format, args...)
}

// errRefused is refuse for functions that return an error.
func errRefused(format string, args ...any) error {
	return fmt.Errorf("forge: refused: "+format, args...)
}

// WriteFile writes content to a repository-relative path, creating parent directories.
func (r *Repo) WriteFile(rel, content string) {
	r.t.Helper()
	p := r.resolve(rel)
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		r.t.Fatalf("forge: mkdir for %s: %v", rel, err)
	}
	if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
		r.t.Fatalf("forge: write %s: %v", rel, err)
	}
}

// ReadFile returns the content of a repository-relative file.
func (r *Repo) ReadFile(rel string) string {
	r.t.Helper()
	b, err := os.ReadFile(r.resolve(rel))
	if err != nil {
		r.t.Fatalf("forge: read %s: %v", rel, err)
	}
	return string(b)
}

// RemoveFile deletes a repository-relative file.
func (r *Repo) RemoveFile(rel string) {
	r.t.Helper()
	if err := os.Remove(r.resolve(rel)); err != nil {
		r.t.Fatalf("forge: remove %s: %v", rel, err)
	}
}

// Chmod sets the mode of a repository-relative file. Fixtures use it for the executable bit.
func (r *Repo) Chmod(rel string, mode os.FileMode) {
	r.t.Helper()
	if err := os.Chmod(r.resolve(rel), mode); err != nil {
		r.t.Fatalf("forge: chmod %s: %v", rel, err)
	}
}

// worktreeNameRE is one plain path segment: it starts with a letter or digit and has no separator, no traversal and
// no leading dash or dot.
var worktreeNameRE = regexp.MustCompile(`^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`)

// worktreeRoot is the directory that holds this repository's worktrees, under TempRoot.
func (r *Repo) worktreeRoot() string { return filepath.Join(r.TempRoot(), "worktrees") }

// worktreeDir returns the destination for a worktree name. The name must be one plain path segment. The worktree
// root must not be a symlink, and nothing may already exist at the destination, which must be a direct child of
// the root.
func (r *Repo) worktreeDir(name string) (string, error) {
	if !worktreeNameRE.MatchString(name) {
		return "", errRefused("worktree name %q is not one plain path segment", name)
	}
	root := r.worktreeRoot()
	if info, err := os.Lstat(root); err == nil && info.Mode()&os.ModeSymlink != 0 {
		return "", errRefused("worktree root %s is a symlink", root)
	}
	dir := filepath.Join(root, name)
	if filepath.Dir(dir) != root {
		return "", errRefused("worktree %q would not be a direct child of %s", name, root)
	}
	if _, err := os.Lstat(dir); err == nil {
		return "", errRefused("worktree destination %s already exists", dir)
	}
	return dir, nil
}

// AddWorktree creates a detached worktree of rev under the worktree root (TempRoot()/worktrees), outside the
// repository, and returns its path. The name is checked before any git command runs. The time-of-check window
// described for resolve applies here too.
func (r *Repo) AddWorktree(name, rev string) (string, error) {
	dir, err := r.worktreeDir(name)
	if err != nil {
		return "", err
	}
	if err := os.MkdirAll(r.worktreeRoot(), 0o755); err != nil {
		return "", fmt.Errorf("forge: create worktree root: %w", err)
	}
	if _, err := r.Git("worktree", "add", "--detach", dir, rev); err != nil {
		return "", err
	}
	return dir, nil
}

// RemoveWorktree removes a worktree created by AddWorktree and prunes its administrative entry. dir must be a
// direct child of the worktree root, and must not be a symlink. Anything else is refused before git runs, so git
// cannot be pointed at a directory outside the root.
func (r *Repo) RemoveWorktree(dir string) error {
	dir = filepath.Clean(dir)
	if filepath.Dir(dir) != r.worktreeRoot() || !worktreeNameRE.MatchString(filepath.Base(dir)) {
		return errRefused("%s is not a worktree directly under %s", dir, r.worktreeRoot())
	}
	if info, err := os.Lstat(dir); err == nil && info.Mode()&os.ModeSymlink != 0 {
		return errRefused("worktree %s is a symlink", dir)
	}
	if _, err := r.Git("worktree", "remove", "--force", dir); err != nil {
		return err
	}
	_, err := r.Git("worktree", "prune")
	return err
}

// SetLine replaces the 1-based line number of a repository-relative file. It creates a real
// content change without any shell, which keeps the forge independent of sed flavours.
func (r *Repo) SetLine(rel string, lineNo int, text string) {
	r.t.Helper()
	lines := strings.Split(strings.TrimSuffix(r.ReadFile(rel), "\n"), "\n")
	if lineNo < 1 || lineNo > len(lines) {
		r.t.Fatalf("forge: %s has %d lines, cannot set line %d", rel, len(lines), lineNo)
	}
	lines[lineNo-1] = text
	r.WriteFile(rel, strings.Join(lines, "\n")+"\n")
}

// Commit stages every change (including deletions) and commits with msg. It returns the new commit SHA.
func (r *Repo) Commit(msg string) string {
	r.t.Helper()
	r.MustGit("add", "-A")
	r.MustGit("commit", "-q", "-m", msg)
	return r.Rev("HEAD")
}

// Rev resolves a revision to a full commit (or tree) SHA.
func (r *Repo) Rev(ref string) string {
	r.t.Helper()
	return r.MustGit("rev-parse", ref)
}

// Branch creates or resets branch name at from, and checks it out.
func (r *Repo) Branch(name, from string) {
	r.t.Helper()
	r.MustGit("checkout", "-q", "-B", name, from)
}

// Checkout switches to an existing branch or revision.
func (r *Repo) Checkout(ref string) {
	r.t.Helper()
	r.MustGit("checkout", "-q", ref)
}

// ConflictingEdits creates branches leftName and rightName from HEAD. Each sets line lineNo of
// rel to its own text and commits the change, so merging one branch into the other conflicts
// (T-0002 AC-4). The caller's checked-out revision is restored afterwards. It returns the two
// commit SHAs in argument order. The branch names must differ from the checked-out branch.
func (r *Repo) ConflictingEdits(rel string, lineNo int, leftName, leftText, rightName, rightText string) (string, string) {
	r.t.Helper()
	prev := r.MustGit("rev-parse", "--abbrev-ref", "HEAD")
	if prev == "HEAD" { // detached: restore the revision itself
		prev = r.Rev("HEAD")
	}
	base := r.Rev("HEAD")
	left := r.editOnBranch(base, rel, lineNo, leftName, leftText)
	right := r.editOnBranch(base, rel, lineNo, rightName, rightText)
	r.Checkout(prev)
	return left, right
}

// editOnBranch creates branch name at from, sets one line of rel and commits it.
func (r *Repo) editOnBranch(from, rel string, lineNo int, name, text string) string {
	r.t.Helper()
	r.Branch(name, from)
	r.SetLine(rel, lineNo, text)
	return r.Commit(name + " edit")
}

// HookDir is an empty directory used as core.hooksPath, so no repository hook can run.
func (r *Repo) HookDir() string { return r.hooks }
