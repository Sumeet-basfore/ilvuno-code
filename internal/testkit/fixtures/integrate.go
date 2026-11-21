package fixtures

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync/atomic"

	"github.com/Sumeet-basfore/ilvuno-code/internal/testkit/forge"
)

// Verdict is the outcome of one sequential integration, in the vocabulary of expected.json.
type Verdict struct {
	Status        string   // ok, conflict, unrelated_histories, check_failed, harness_refused, operational
	Exit          int      // exit class: 0 ok, 1 check, 2 operational, 3 conflict or unrelated, 4 refused, 5 cleanup
	ConflictStep  int      // 1-based step that conflicted or was unrelated; 0 otherwise
	ConflictPaths []string // conflicted paths in index order
	Candidate     string   // candidate commit SHA after the last clean step (base when no step merged)
	CleanupFailed bool     // true when worktree removal was skipped or failed
	WorktreeDir   string   // the leftover directory when cleanup was skipped
}

// Options controls fault injection. The zero value is a normal run.
type Options struct {
	// FailCleanup skips worktree removal, leaving an orphaned directory (F9 fault injection).
	FailCleanup bool
}

var worktreeSeq atomic.Uint64

// builtinDriverValues are merge-driver values Git treats as built in (oracle.py).
var builtinDriverValues = map[string]bool{"true": true, ":": true, "binary": true, "text": true, "union": true}

// refusedFeatures lists the executable features the reference refuses before any merge (oracle.py
// REFUSE_PREFIXES). It reads the local configuration with -z, so each key and value is parsed exactly,
// without depending on how git formats its human-readable listing.
func refusedFeatures(r *forge.Repo) ([]string, error) {
	out, err := r.Git("config", "--local", "--list", "-z")
	if err != nil {
		return nil, err
	}
	var reasons []string
	for _, rec := range strings.Split(out, "\x00") {
		if rec == "" {
			continue
		}
		// With -z, the key and its value are separated by a newline, and the records by NUL.
		key, val, _ := strings.Cut(rec, "\n")
		switch {
		case strings.HasPrefix(key, "filter."), strings.HasPrefix(key, "diff."):
			reasons = append(reasons, key)
		case key == "merge.default":
			reasons = append(reasons, "merge.default configured")
		case strings.HasPrefix(key, "merge.") && strings.HasSuffix(key, ".driver") && !builtinDriverValues[val]:
			reasons = append(reasons, "custom merge driver: "+key)
		}
	}
	return reasons, nil
}

// Integrate performs the sequential reference integration of sources onto base, the way the SP-1R3
// oracle does: refuse configured executable features, then one `merge --no-ff` per source in a detached
// worktree, with identity ilvuno and the base commit time. The F7 check runs on the candidate. The
// worktree is always removed unless fault injection says otherwise.
func Integrate(r *forge.Repo, base string, sources []string, opts Options) (Verdict, error) {
	reasons, err := refusedFeatures(r)
	if err != nil {
		return Verdict{}, fmt.Errorf("list config: %w", err)
	}
	if len(reasons) > 0 {
		return Verdict{Status: "harness_refused", Exit: 4, Candidate: ""}, nil
	}

	name := fmt.Sprintf("integrate-%d", worktreeSeq.Add(1))
	wt, err := r.AddWorktree(name, base)
	if err != nil {
		return Verdict{}, fmt.Errorf("add worktree: %w", err)
	}
	ts, err := r.Git("log", "-1", "--format=%ct", base)
	if err != nil {
		return Verdict{}, fmt.Errorf("base time: %w", err)
	}

	v := Verdict{Status: "ok", Candidate: base}
	runErr := integrateSteps(r, wt, ts, sources, &v)
	if runErr == nil && v.Status == "ok" {
		checkFailed, cerr := runCheck(wt)
		if cerr != nil {
			runErr = cerr
		} else if checkFailed {
			v.Status, v.Exit = "check_failed", 1
		}
	}

	// Cleanup runs on every path, including errors, so no worktree is silently left behind.
	if opts.FailCleanup {
		v.CleanupFailed, v.WorktreeDir = true, wt
		if v.Status == "ok" {
			v.Exit = 5
		}
		return v, runErr
	}
	if err := r.RemoveWorktree(wt); err != nil {
		v.CleanupFailed = true
		if v.Status == "ok" {
			v.Exit = 5
		}
	}
	return v, runErr
}

// integrateSteps merges each source in order and records the first conflict or unrelated history.
func integrateSteps(r *forge.Repo, wt, ts string, sources []string, v *Verdict) error {
	for i, src := range sources {
		step := i + 1
		if _, err := r.GitIn(wt, nil, "merge-base", "--is-ancestor", src, "HEAD"); err == nil {
			continue // already integrated: skip without an empty merge commit
		}
		if _, err := r.GitIn(wt, nil, "merge-base", "HEAD", src); err != nil {
			v.Status, v.Exit, v.ConflictStep = "unrelated_histories", 3, step
			return nil
		}
		msg := fmt.Sprintf("ilvuno: integrate %s (step %d/%d)", src[:12], step, len(sources))
		env := []string{
			"GIT_AUTHOR_NAME=ilvuno", "GIT_AUTHOR_EMAIL=ilvuno@localhost",
			"GIT_COMMITTER_NAME=ilvuno", "GIT_COMMITTER_EMAIL=ilvuno@localhost",
			"GIT_AUTHOR_DATE=@" + ts + " +0000", "GIT_COMMITTER_DATE=@" + ts + " +0000",
		}
		_, err := r.GitIn(wt, env, "merge", "--no-ff", "--no-edit", "-m", msg, src)
		if err == nil {
			head, herr := r.GitIn(wt, nil, "rev-parse", "HEAD")
			if herr != nil {
				return fmt.Errorf("rev-parse after step %d: %w", step, herr)
			}
			v.Candidate = head
			continue
		}
		paths, perr := unmergedPaths(r, wt)
		if perr != nil {
			return perr
		}
		if len(paths) == 0 {
			v.Status, v.Exit, v.ConflictStep = "operational", 2, step
			return nil
		}
		v.Status, v.Exit, v.ConflictStep, v.ConflictPaths = "conflict", 3, step, paths
		return nil
	}
	return nil
}

// unmergedPaths returns the distinct conflicted paths of the worktree in index order.
func unmergedPaths(r *forge.Repo, wt string) ([]string, error) {
	out, err := r.GitIn(wt, nil, "ls-files", "-u", "-z")
	if err != nil {
		return nil, fmt.Errorf("ls-files -u: %w", err)
	}
	var paths []string
	seen := map[string]bool{}
	for _, entry := range strings.Split(out, "\x00") {
		if entry == "" {
			continue
		}
		_, path, ok := strings.Cut(entry, "\t")
		if !ok {
			path = entry
		}
		if !seen[path] {
			seen[path] = true
			paths = append(paths, path)
		}
	}
	return paths, nil
}

// runCheck runs check.sh in the candidate worktree when present, and reports whether it failed. The
// check is the F7 case's own check script (part of the reference behaviour, not a driver or a hook). It
// runs with a minimal environment.
func runCheck(wt string) (bool, error) {
	if _, err := os.Stat(filepath.Join(wt, "check.sh")); os.IsNotExist(err) {
		return false, nil
	} else if err != nil {
		return false, fmt.Errorf("stat check.sh: %w", err)
	}
	cmd := exec.Command("sh", "./check.sh")
	cmd.Dir = wt
	cmd.Env = []string{"PATH=" + os.Getenv("PATH"), "LANG=C", "LC_ALL=C"}
	if err := cmd.Run(); err != nil {
		if _, ok := err.(*exec.ExitError); ok {
			return true, nil
		}
		return false, fmt.Errorf("run check.sh: %w", err)
	}
	return false, nil
}
