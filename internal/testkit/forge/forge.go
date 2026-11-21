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
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
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
type Repo struct {
	t     testing.TB
	Path  string
	home  string
	tmp   string
	hooks string
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

// environ is the complete environment for git children. It does not inherit the caller's environment,
// except PATH, so caller configuration variables cannot leak in.
func (r *Repo) environ() []string {
	return []string{
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
	}
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

// resolve returns the absolute path for a repository-relative path. It rejects paths that
// would leave the repository, so the forge cannot write outside its t.TempDir().
func (r *Repo) resolve(rel string) string {
	r.t.Helper()
	if !filepath.IsLocal(rel) {
		r.t.Fatalf("forge: path %q escapes the repository", rel)
	}
	return filepath.Join(r.Path, rel)
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

// AddWorktree creates a detached worktree of rev in a sibling directory outside the repository
// and returns its path. Worktrees are not part of the repository's own state.
func (r *Repo) AddWorktree(name, rev string) (string, error) {
	dir := filepath.Join(filepath.Dir(r.Path), "worktrees", name)
	if err := os.MkdirAll(filepath.Dir(dir), 0o755); err != nil {
		return "", fmt.Errorf("forge: create worktree parent: %w", err)
	}
	if _, err := r.Git("worktree", "add", "--detach", dir, rev); err != nil {
		return "", err
	}
	return dir, nil
}

// RemoveWorktree removes a worktree created by AddWorktree and prunes its administrative entry.
func (r *Repo) RemoveWorktree(dir string) error {
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
