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
}

func TestHostileConfigIsIgnored(t *testing.T) {
	// Arrange: a baseline build, then a build with a hostile global config pointed at
	// by HOME, GIT_CONFIG_GLOBAL and GIT_CONFIG_SYSTEM (AC-2).
	baseline := New(t)
	buildSample(baseline)
	want := baseline.Snapshot()

	hostile := t.TempDir()
	hostileCfg := filepath.Join(hostile, "gitconfig")
	cfg := "[user]\n\tname = Hostile\n\temail = hostile@example.invalid\n" +
		"[commit]\n\tgpgsign = true\n[gpg]\n\tprogram = /bin/false\n" +
		"[core]\n\thooksPath = /nonexistent-hooks\n[init]\n\tdefaultBranch = hostile\n" +
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

func TestConflictHelperProducesRealConflict(t *testing.T) {
	// Arrange: two branches edit the same line differently.
	r := New(t)
	r.WriteFile("a.txt", "x\ny\nz\n")
	r.Commit("base")
	r.Branch("left", "main")
	r.SetLine("a.txt", 2, "left")
	r.Commit("left")
	r.Branch("right", "main")
	r.SetLine("a.txt", 2, "right")
	r.Commit("right")
	r.Checkout("left")

	// Act
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
	// Arrange: a baseline snapshot.
	r := New(t)
	buildSample(r)
	base := r.Snapshot()

	cases := []struct {
		name   string
		mutate func()
		want   string
	}{
		{"new ref", func() { r.MustGit("update-ref", "refs/heads/extra", "HEAD") }, "ref refs/heads/extra"},
		{"staged change", func() {
			r.WriteFile("staged.txt", "s\n")
			r.MustGit("add", "staged.txt")
		}, "index digest"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			// Arrange: restore the baseline before each mutation.
			r.MustGit("reset", "-q", "--hard", base.Head)
			_, _ = r.Git("update-ref", "-d", "refs/heads/extra") // absent before the first mutation
			r.MustGit("clean", "-fdq")
			tc.mutate()

			// Act
			diff := base.Diff(r.Snapshot())

			// Assert: the diff names the changed field.
			if !strings.Contains(diff, tc.want) {
				t.Fatalf("diff %q does not name %q", diff, tc.want)
			}
		})
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
