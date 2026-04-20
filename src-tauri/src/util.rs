//! Path and filesystem helpers.

use std::path::{Path, PathBuf};

pub trait ExtExt {
    /// Return the file's extension as a lowercase owned string, if any.
    fn ext_lower(&self) -> Option<String>;
}

impl ExtExt for Path {
    fn ext_lower(&self) -> Option<String> {
        self.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
    }
}

impl ExtExt for PathBuf {
    fn ext_lower(&self) -> Option<String> {
        self.as_path().ext_lower()
    }
}

/// Given a candidate output path, return a path that doesn't clash with
/// an existing file. If `file.png` exists, try `file (1).png`, `file (2).png`, etc.
pub fn unique_sibling_path(candidate: &Path) -> PathBuf {
    if !candidate.exists() {
        return candidate.to_path_buf();
    }
    let parent = candidate.parent().unwrap_or(Path::new("."));
    let stem = candidate
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = candidate
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    for i in 1..=9999 {
        let next = if ext.is_empty() {
            parent.join(format!("{stem} ({i})"))
        } else {
            parent.join(format!("{stem} ({i}).{ext}"))
        };
        if !next.exists() {
            return next;
        }
    }
    // Extreme fallback — should never hit this in practice.
    candidate.to_path_buf()
}
