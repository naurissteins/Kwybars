mod compat;
mod load;
mod parse;
mod resolve;

use std::fs;
use std::path::{Path, PathBuf};

use super::{Parsed, parse as parse_config};

/// parses text that must be valid
fn parse_ok(raw: &str) -> Parsed {
    match parse_config(raw) {
        Ok(parsed) => parsed,
        Err(err) => panic!("config should parse, got: {err}"),
    }
}

/// parses text that must fail and returns the rendered error
fn parse_err(raw: &str) -> String {
    match parse_config(raw) {
        Ok(parsed) => panic!("config should fail, got: {parsed:?}"),
        Err(err) => err.to_string(),
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-5,
        "expected {expected}, got {actual}"
    );
}

/// a fresh directory under the system temp dir, removed on drop
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("kwybars-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        if let Err(err) = fs::create_dir_all(&path) {
            panic!("create {}: {err}", path.display());
        }
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// writes `raw` to `name`, creating parent directories
    pub(crate) fn write(&self, name: &str, raw: &str) -> PathBuf {
        let path = self.0.join(name);
        if let Some(parent) = path.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            panic!("create {}: {err}", parent.display());
        }
        if let Err(err) = fs::write(&path, raw) {
            panic!("write {}: {err}", path.display());
        }
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
