//! On-disk layout under `ILVUNO_HOME`. Tests always pass a temp directory; the CLI reads the env var.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db_path(&self) -> PathBuf {
        self.root.join("ilvuno.db")
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.root.join("runs").join(run_id)
    }

    pub fn threads_dir(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("threads")
    }

    pub fn artifacts_dir(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("artifacts")
    }

    pub fn board_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("board.md")
    }
}
