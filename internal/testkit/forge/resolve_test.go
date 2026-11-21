package forge

import (
	"os"
	"path/filepath"
	"testing"
)

// TestWriteFileCreatesNestedPathsAsDirectories guards a defect found during the containment fix: resolve once returned
// the prefix at the first missing component, so WriteFile("dir/file") wrote a file named "dir". The write must land at
// the full path, with every missing parent created as a directory.
func TestWriteFileCreatesNestedPathsAsDirectories(t *testing.T) {
	r := New(t)
	r.WriteFile("config/.keep", "")
	r.WriteFile("a/b/c.txt", "nested\n")

	for _, dir := range []string{"config", "a", "a/b"} {
		info, err := os.Lstat(filepath.Join(r.Path, dir))
		if err != nil || !info.IsDir() {
			t.Fatalf("%s is not a directory: %v %v", dir, info, err)
		}
	}
	if got := r.ReadFile("a/b/c.txt"); got != "nested\n" {
		t.Fatalf("a/b/c.txt = %q", got)
	}
	if got := r.ReadFile("config/.keep"); got != "" {
		t.Fatalf("config/.keep = %q", got)
	}
}
