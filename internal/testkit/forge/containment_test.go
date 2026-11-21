package forge

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// refusedPrefix is the start of every containment refusal the forge reports. A git error is not a refusal, so the
// tests require this text, not merely a failure, before they accept that containment held.
const refusedPrefix = "forge: refused"

// sentinel creates an external directory, outside the repository and outside its temporary root, holding one file
// whose content must never change. It returns the directory, the file, and a snapshot of the directory listing.
func sentinel(t *testing.T) (dir, file, before string) {
	t.Helper()
	dir = t.TempDir()
	file = filepath.Join(dir, "sentinel.txt")
	if err := os.WriteFile(file, []byte("untouched\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	return dir, file, listing(t, dir)
}

// listing returns the names and contents of every entry under dir, for before-and-after comparison.
func listing(t *testing.T, dir string) string {
	t.Helper()
	var b strings.Builder
	err := filepath.WalkDir(dir, func(p string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		info, err := os.Lstat(p)
		if err != nil {
			return err
		}
		b.WriteString(p + " " + info.Mode().String())
		if info.Mode().IsRegular() {
			content, err := os.ReadFile(p)
			if err != nil {
				return err
			}
			b.WriteString(" " + string(content))
		}
		b.WriteString("\n")
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return b.String()
}

// refused runs fn with the forge's failure handler replaced, and reports whether the forge refused the operation
// (called Fatalf) and what it reported.
func refused(t *testing.T, r *Repo, fn func()) (bool, string) {
	t.Helper()
	fake := &fakeTB{TB: t}
	r.t = fake
	defer func() { r.t = t }()
	func() {
		defer func() { _ = recover() }()
		fn()
	}()
	return fake.fatal != "", fake.fatal
}

func TestWriteFileRefusesSymlinkedFinalComponent(t *testing.T) {
	r := New(t)
	outDir, outFile, before := sentinel(t)
	if err := os.Symlink(outFile, filepath.Join(r.Path, "link.txt")); err != nil {
		t.Fatal(err)
	}

	ok, msg := refused(t, r, func() { r.WriteFile("link.txt", "escaped\n") })

	if !ok || !strings.Contains(msg, "symlink") {
		t.Fatalf("write through a symlink was not refused: refused=%v msg=%q", ok, msg)
	}
	if got := listing(t, outDir); got != before {
		t.Fatalf("external sentinel changed:\nbefore %s\nafter  %s", before, got)
	}
}

func TestWriteFileRefusesSymlinkedIntermediateComponent(t *testing.T) {
	r := New(t)
	outDir, _, before := sentinel(t)
	if err := os.Symlink(outDir, filepath.Join(r.Path, "dir")); err != nil {
		t.Fatal(err)
	}

	ok, msg := refused(t, r, func() { r.WriteFile("dir/new/escaped.txt", "escaped\n") })

	if !ok || !strings.Contains(msg, "symlink") {
		t.Fatalf("write under a symlinked directory was not refused: refused=%v msg=%q", ok, msg)
	}
	if got := listing(t, outDir); got != before {
		t.Fatalf("external directory changed:\nbefore %s\nafter  %s", before, got)
	}
	if _, err := os.Stat(filepath.Join(outDir, "new")); err == nil {
		t.Fatal("MkdirAll created a directory outside the repository")
	}
}

func TestReadAndRemoveRefuseSymlinkedPaths(t *testing.T) {
	r := New(t)
	outDir, outFile, before := sentinel(t)
	if err := os.Symlink(outFile, filepath.Join(r.Path, "link.txt")); err != nil {
		t.Fatal(err)
	}

	if ok, _ := refused(t, r, func() { r.RemoveFile("link.txt") }); !ok {
		t.Fatal("RemoveFile through a symlink was not refused")
	}
	if ok, _ := refused(t, r, func() { r.ReadFile("link.txt") }); !ok {
		t.Fatal("ReadFile through a symlink was not refused")
	}
	if ok, _ := refused(t, r, func() { r.Chmod("link.txt", 0o777) }); !ok {
		t.Fatal("Chmod through a symlink was not refused")
	}
	if got := listing(t, outDir); got != before {
		t.Fatalf("external sentinel changed:\nbefore %s\nafter  %s", before, got)
	}
}

func TestWriteFileRefusesTraversalAndAbsolutePaths(t *testing.T) {
	r := New(t)
	outDir, _, before := sentinel(t)
	for _, rel := range []string{"../escape.txt", "a/../../escape.txt", filepath.Join(outDir, "abs.txt")} {
		if ok, _ := refused(t, r, func() { r.WriteFile(rel, "x") }); !ok {
			t.Errorf("WriteFile(%q) was not refused", rel)
		}
	}
	if got := listing(t, outDir); got != before {
		t.Fatalf("external sentinel changed:\nbefore %s\nafter  %s", before, got)
	}
}

func TestAddWorktreeRefusesTraversalNames(t *testing.T) {
	r := New(t)
	// The temporary root is the parent of the repository. A traversing name aims at a sibling of the root.
	target := filepath.Join(r.TempRoot(), "escape-target")
	if err := os.Mkdir(target, 0o755); err != nil {
		t.Fatal(err)
	}
	before := listing(t, target)
	wtParent := filepath.Join(r.TempRoot(), "worktrees")

	for _, name := range []string{"../escape-target", "..", ".", "a/b", "/" + target, "", ".hidden", "-option"} {
		if _, err := r.AddWorktree(name, "HEAD"); err == nil || !strings.Contains(err.Error(), refusedPrefix) {
			t.Errorf("AddWorktree(%q) was not refused by the forge: %v", name, err)
		}
	}
	if got := listing(t, target); got != before {
		t.Fatalf("a traversing worktree name wrote outside the root:\nbefore %s\nafter  %s", before, got)
	}
	if _, err := os.Stat(wtParent); err == nil {
		if entries, _ := os.ReadDir(wtParent); len(entries) != 0 {
			t.Fatalf("a refused worktree name left entries in %s", wtParent)
		}
	}
}

func TestAddWorktreeRefusesSymlinkedWorktreeRoot(t *testing.T) {
	r := New(t)
	outDir, _, before := sentinel(t)
	if err := os.Symlink(outDir, filepath.Join(r.TempRoot(), "worktrees")); err != nil {
		t.Fatal(err)
	}

	if _, err := r.AddWorktree("integrate-1", "HEAD"); err == nil || !strings.Contains(err.Error(), refusedPrefix) {
		t.Fatalf("AddWorktree through a symlinked root was not refused by the forge: %v", err)
	}
	if got := listing(t, outDir); got != before {
		t.Fatalf("a worktree was created through a symlinked root:\nbefore %s\nafter  %s", before, got)
	}
}

func TestRemoveWorktreeRefusesPathsOutsideTheRoot(t *testing.T) {
	r := New(t)
	outDir, _, before := sentinel(t)

	if err := r.RemoveWorktree(outDir); err == nil || !strings.Contains(err.Error(), refusedPrefix) {
		t.Fatalf("RemoveWorktree of an external directory was not refused by the forge: %v", err)
	}
	if got := listing(t, outDir); got != before {
		t.Fatalf("RemoveWorktree touched an external directory:\nbefore %s\nafter  %s", before, got)
	}
}

func TestAddWorktreeAcceptsAPlainNameInsideTheRoot(t *testing.T) {
	// Control: the containment checks must not refuse a normal worktree.
	r := New(t)
	r.WriteFile("a.txt", "one\n")
	r.Commit("one")
	dir, err := r.AddWorktree("integrate-1", "HEAD")
	if err != nil {
		t.Fatalf("a plain worktree name was refused: %v", err)
	}
	if filepath.Dir(dir) != filepath.Join(r.TempRoot(), "worktrees") {
		t.Fatalf("worktree %s is outside the worktree root", dir)
	}
	if err := r.RemoveWorktree(dir); err != nil {
		t.Fatalf("RemoveWorktree of a plain worktree: %v", err)
	}
}

// The DRIVER_MARKER tests. A marker is a file that a configured program writes, so its path must stay inside the
// temporary root. A string prefix check is not enough: "<root>/../x" has the root as its prefix, and a symlink at
// the marker path leads outside. Each case below must be refused at the time the variable is set.

func TestSetEnvRefusesMarkerTraversingOutOfTheRoot(t *testing.T) {
	r := New(t)
	value := r.TempRoot() + "/../" + filepath.Base(r.TempRoot()) + "-escape"
	if ok, msg := refused(t, r, func() { r.SetEnv("DRIVER_MARKER", value) }); !ok || !strings.Contains(msg, refusedPrefix) {
		t.Fatalf("a marker path that traverses out of the root was not refused: refused=%v msg=%q", ok, msg)
	}
}

func TestSetEnvRefusesMarkerThatIsASymlinkOutsideTheRoot(t *testing.T) {
	r := New(t)
	outDir := t.TempDir()
	victim := filepath.Join(outDir, "victim.txt")
	if err := os.WriteFile(victim, []byte("victim\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	marker := filepath.Join(r.TempRoot(), "driver-marker")
	if err := os.Symlink(victim, marker); err != nil {
		t.Fatal(err)
	}
	before := listing(t, outDir)

	if ok, msg := refused(t, r, func() { r.SetEnv("DRIVER_MARKER", marker) }); !ok || !strings.Contains(msg, "symlink") {
		t.Fatalf("a symlinked marker was not refused: refused=%v msg=%q", ok, msg)
	}
	if got := listing(t, outDir); got != before {
		t.Fatalf("the external victim changed:\nbefore %s\nafter  %s", before, got)
	}
}

func TestSetEnvRefusesMarkerInASymlinkedDirectory(t *testing.T) {
	r := New(t)
	outDir := t.TempDir()
	if err := os.Symlink(outDir, filepath.Join(r.TempRoot(), "dir")); err != nil {
		t.Fatal(err)
	}
	if ok, msg := refused(t, r, func() { r.SetEnv("DRIVER_MARKER", filepath.Join(r.TempRoot(), "dir", "marker")) }); !ok || !strings.Contains(msg, "symlink") {
		t.Fatalf("a marker under a symlinked directory was not refused: refused=%v msg=%q", ok, msg)
	}
}

func TestSetEnvRefusesTheRootItselfAndOtherVariables(t *testing.T) {
	r := New(t)
	if ok, _ := refused(t, r, func() { r.SetEnv("DRIVER_MARKER", r.TempRoot()) }); !ok {
		t.Fatal("the temporary root itself was accepted as a marker")
	}
	for _, key := range []string{"PATH", "HOME", "GIT_CONFIG_GLOBAL", "GIT_ALLOW_PROTOCOL"} {
		if ok, _ := refused(t, r, func() { r.SetEnv(key, filepath.Join(r.TempRoot(), "x")) }); !ok {
			t.Errorf("SetEnv accepted %s", key)
		}
	}
}

func TestSetEnvAcceptsAPlainMarkerInsideTheRoot(t *testing.T) {
	// Control: a plain marker path inside the root is accepted.
	r := New(t)
	if ok, msg := refused(t, r, func() { r.SetEnv("DRIVER_MARKER", filepath.Join(r.TempRoot(), "driver-marker")) }); ok {
		t.Fatalf("a plain marker path was refused: %s", msg)
	}
}
