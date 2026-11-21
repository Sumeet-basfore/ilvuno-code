package forge

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

// buildSample builds the same two-branch history in r every time. It is the "one spec" of the
// determinism tests (T-0002 AC-1).
func buildSample(r *Repo) {
	r.WriteFile("shared.txt", "line1\nline2\nline3\n")
	r.WriteFile("stable.txt", "base\n")
	r.Commit("base")
	r.Branch("s1", "main")
	r.SetLine("shared.txt", 2, "s1-line2")
	r.WriteFile("s1.txt", "one\n")
	r.Commit("s1")
	r.Checkout("main")
}

func TestTwoBuildsOfOneSpecAreIdentical(t *testing.T) {
	// Arrange + Act: two independent repositories built from the same steps.
	a := New(t)
	buildSample(a)
	b := New(t)
	buildSample(b)

	// Assert: identical refs, HEAD, tree and index (AC-1).
	sa, sb := a.Snapshot(), b.Snapshot()
	if !sa.Equal(sb) {
		t.Fatalf("snapshots differ: %s", sa.Diff(sb))
	}
	if sa.Head == "" || sa.Tree == "" {
		t.Fatalf("snapshot missing HEAD or tree: %+v", sa)
	}
	// Evidence: both dumps, which the T-0002 evidence requirements ask for.
	t.Logf("snapshot A: head=%s tree=%s index=%s refs=%v", sa.Head, sa.Tree, sa.Index, sa.Refs)
	t.Logf("snapshot B: head=%s tree=%s index=%s refs=%v", sb.Head, sb.Tree, sb.Index, sb.Refs)
}

func TestHostileConfigIsIgnored(t *testing.T) {
	// Arrange: a baseline build, then a build with a hostile global config pointed at
	// by HOME, GIT_CONFIG_GLOBAL and GIT_CONFIG_SYSTEM (AC-2).
	baseline := New(t)
	buildSample(baseline)
	want := baseline.Snapshot()
	if len(want.Head) != 40 {
		t.Fatalf("baseline HEAD %q is not a SHA-1 object name; the forge environment is not in effect", want.Head)
	}

	hostile := t.TempDir()
	hostileCfg := filepath.Join(hostile, "gitconfig")
	cfg := "[user]\n\tname = Hostile\n\temail = hostile@example.invalid\n" +
		"[commit]\n\tgpgsign = true\n[gpg]\n\tprogram = /bin/false\n" +
		"[core]\n\thooksPath = /nonexistent-hooks\n[init]\n\tdefaultBranch = hostile\n" +
		"\tdefaultObjectFormat = sha256\n" + // not overridden by the forge: a leak would change every SHA
		"[merge \"upper\"]\n\tdriver = touch " + filepath.Join(hostile, "ran") + "\n"
	if err := os.WriteFile(hostileCfg, []byte(cfg), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Setenv("HOME", hostile)
	t.Setenv("GIT_CONFIG_GLOBAL", hostileCfg)
	t.Setenv("GIT_CONFIG_SYSTEM", hostileCfg)
	t.Setenv("XDG_CONFIG_HOME", hostile)

	// Act
	got := New(t)
	buildSample(got)

	// Assert: the hostile settings changed nothing, and the driver marker was never created.
	if diff := want.Diff(got.Snapshot()); diff != "" {
		t.Fatalf("hostile config changed the build: %s", diff)
	}
	if _, err := os.Stat(filepath.Join(hostile, "ran")); err == nil {
		t.Fatal("hostile merge driver executed")
	}
}

func TestCommitDatesAreFixed(t *testing.T) {
	// Arrange + Act
	r := New(t)
	buildSample(r)

	// Assert: every commit carries the exact reference date and identity (AC-3).
	for _, format := range []string{"%ct", "%at"} {
		got := r.MustGit("log", "-1", "--format="+format)
		n, err := strconv.ParseInt(got, 10, 64)
		if err != nil {
			t.Fatalf("parse %s: %v", got, err)
		}
		if n != DateUnix {
			t.Fatalf("log %s = %d, want %d", format, n, DateUnix)
		}
	}
	if got := r.MustGit("log", "-1", "--format=%an <%ae> / %cn <%ce>"); got != Identity+" <"+Email+"> / "+Identity+" <"+Email+">" {
		t.Fatalf("identity = %q", got)
	}
}

func TestConflictingEditsProduceRealConflict(t *testing.T) {
	// Arrange: the helper creates two branches that edit the same line differently.
	r := New(t)
	r.WriteFile("a.txt", "x\ny\nz\n")
	r.Commit("base")
	base := r.Rev("HEAD")
	left, right := r.ConflictingEdits("a.txt", 2, "left", "left-text", "right", "right-text")

	// Assert: both branches exist, differ from the base and from each other, and HEAD is untouched.
	if left == base || right == base || left == right {
		t.Fatalf("edits did not create distinct commits: base %s left %s right %s", base, left, right)
	}
	if got := r.Rev("left"); got != left {
		t.Fatalf("branch left = %s, want %s", got, left)
	}
	if got := r.Rev("right"); got != right {
		t.Fatalf("branch right = %s, want %s", got, right)
	}
	if got := r.Rev("HEAD"); got != base {
		t.Fatalf("HEAD moved to %s, want %s", got, base)
	}

	// Act: merge right into left.
	r.Checkout("left")
	_, err := r.Git("merge", "--no-edit", "right")

	// Assert: the merge fails and leaves an unmerged path (AC-4).
	if err == nil {
		t.Fatal("merge succeeded; expected a conflict")
	}
	if unmerged := r.MustGit("ls-files", "-u"); unmerged == "" {
		t.Fatal("no unmerged entries after a conflicting merge")
	}
	r.MustGit("merge", "--abort")
}

func TestConflictingEditsRefusesMissingLine(t *testing.T) {
	// Arrange: a one-line file has no line 5.
	r := New(t)
	r.WriteFile("a.txt", "only\n")
	r.Commit("base")
	fake := &fakeTB{TB: t}
	r.t = fake

	// Act: the helper must stop rather than silently create an edit elsewhere.
	func() {
		defer func() { _ = recover() }()
		r.ConflictingEdits("a.txt", 5, "left", "l", "right", "r")
	}()

	// Assert
	if !strings.Contains(fake.fatal, "cannot set line 5") {
		t.Fatalf("missing line was not refused; fatal = %q", fake.fatal)
	}
}

func TestNetworkIsRefused(t *testing.T) {
	// Arrange: a remote that would need a network transport (AC-6).
	r := New(t)
	buildSample(r)
	r.MustGit("remote", "add", "origin", "https://unreachable.invalid/ilvuno.git")

	// Act
	_, err := r.Git("fetch", "origin")

	// Assert: the forge environment refuses every protocol.
	if err == nil {
		t.Fatal("fetch succeeded; the forge must not reach the network")
	}
	if !strings.Contains(err.Error(), "not allowed") {
		t.Fatalf("error %q does not show the protocol refusal", err)
	}
}

func TestGitErrorNamesArgvAndStderr(t *testing.T) {
	r := New(t)

	_, err := r.Git("no-such-subcommand", "--flag")

	if err == nil {
		t.Fatal("expected an error")
	}
	for _, want := range []string{"no-such-subcommand --flag", "not a git command"} {
		if !strings.Contains(err.Error(), want) {
			t.Fatalf("error %q does not contain %q", err, want)
		}
	}
}

// fakeTB records a fatal call instead of stopping the test, so the path check can be exercised.
type fakeTB struct {
	testing.TB
	fatal string
}

func (f *fakeTB) Helper()                        {}
func (f *fakeTB) Fatalf(format string, a ...any) { f.fatal = fmt.Sprintf(format, a...); panic(f) }
func (f *fakeTB) Cleanup(func())                 {}
func (f *fakeTB) TempDir() string                { return "" }
func (f *fakeTB) Fatal(a ...any)                 { f.fatal = fmt.Sprint(a...); panic(f) }
func (f *fakeTB) Name() string                   { return "fake" }
func (f *fakeTB) Setenv(key, value string)       {}
func (f *fakeTB) Errorf(format string, a ...any) {}

func TestWritesCannotEscapeTheRepository(t *testing.T) {
	cases := []struct {
		name string
		rel  string
	}{
		{"parent traversal", "../escape.txt"},
		{"absolute path", "/tmp/escape.txt"},
		{"traversal inside a directory", "dir/../../escape.txt"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			// Arrange: a real repository, observed through a fake TB that records the fatal.
			r := New(t)
			fake := &fakeTB{TB: t}
			r.t = fake

			// Act
			func() {
				defer func() { _ = recover() }()
				r.WriteFile(tc.rel, "x")
			}()

			// Assert
			if !strings.Contains(fake.fatal, "escapes the repository") {
				t.Fatalf("WriteFile(%q) was not refused; fatal = %q", tc.rel, fake.fatal)
			}
			if _, err := os.Stat(filepath.Join(filepath.Dir(r.Path), "escape.txt")); err == nil {
				t.Fatal("a file was written outside the repository")
			}
		})
	}
}

func TestSnapshotDiffNamesEachChangedField(t *testing.T) {
	cases := []struct {
		name   string
		mutate func(r *Repo)
		want   string
	}{
		{"new ref", func(r *Repo) { r.MustGit("update-ref", "refs/heads/extra", "HEAD") }, "ref refs/heads/extra"},
		{"new commit moves head", func(r *Repo) {
			r.WriteFile("later.txt", "l\n")
			r.Commit("later")
		}, "head"},
		{"staged change", func(r *Repo) {
			r.WriteFile("staged.txt", "s\n")
			r.MustGit("add", "staged.txt")
		}, "index digest"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			// Arrange: a fresh repository and baseline for each mutation, so cases share no state.
			r := New(t)
			buildSample(r)
			base := r.Snapshot()

			// Act
			tc.mutate(r)
			diff := base.Diff(r.Snapshot())

			// Assert: the diff names the changed field.
			if !strings.Contains(diff, tc.want) {
				t.Fatalf("diff %q does not name %q", diff, tc.want)
			}
		})
	}
}

func TestDifferentSpecsProduceDifferentSnapshots(t *testing.T) {
	// Arrange: the sample spec, and the same spec with one branch line changed.
	a := New(t)
	buildSample(a)
	b := New(t)
	buildSample(b)
	b.Branch("s1", "main")
	b.SetLine("shared.txt", 2, "s1-changed")
	b.Commit("s1 changed")
	b.Checkout("main")

	// Act
	diff := a.Snapshot().Diff(b.Snapshot())

	// Assert: the difference is reported on the branch that changed (and not on main).
	if !strings.Contains(diff, "ref refs/heads/s1") {
		t.Fatalf("diff %q does not name refs/heads/s1", diff)
	}
	if strings.Contains(diff, "ref refs/heads/main") {
		t.Fatalf("diff %q names refs/heads/main, which did not change", diff)
	}
}

func TestSelfTestsBuildTenFixturesWithinTwoSeconds(t *testing.T) {
	// AC-5: ten repositories of the same shape in under two seconds.
	const count = 10
	start := time.Now()
	for i := 0; i < count; i++ {
		r := New(t)
		buildSample(r)
	}
	if elapsed := time.Since(start); elapsed > 2*time.Second {
		t.Fatalf("building %d repositories took %v, budget 2s", count, elapsed)
	}
}
