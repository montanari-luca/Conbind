//! Space analyzer: one directory level at a time.
//!
//! Listing a single level and sizing each child keeps the cost proportional to
//! what the user is looking at, instead of walking the whole disk up front to
//! build a tree they will mostly never open. The frontend keeps the listings
//! it has already loaded, so going back up is free.

use super::sizing;
use crate::error::AppError;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Children returned per directory. The long tail of tiny entries is noise in
/// a "what is using my space" view; the total still covers all of them.
const MAX_ENTRIES: usize = 300;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedDir {
    pub path: String,
    /// `None` at the home directory: the analyzer never goes above it.
    pub parent: Option<String>,
    pub total_bytes: u64,
    pub entries: Vec<AnalyzedEntry>,
}

/// Lists `path` (the home directory when `None`) with each child sized and the
/// biggest first. The path must be a directory inside `home`.
pub fn analyze_dir(home: &Path, path: Option<&str>) -> Result<AnalyzedDir, AppError> {
    let home = dunce::canonicalize(home)
        .map_err(|e| AppError::Other(format!("Could not resolve home directory: {e}")))?;
    let dir = match path {
        None => home.clone(),
        Some(p) => {
            let p = Path::new(p);
            if !p.is_absolute() {
                return Err(AppError::Validation(format!(
                    "Path must be absolute: {}",
                    p.display()
                )));
            }
            dunce::canonicalize(p)
                .map_err(|e| AppError::Other(format!("Could not open '{}': {e}", p.display())))?
        }
    };
    if !dir.starts_with(&home) {
        return Err(AppError::Permission(format!(
            "'{}' is outside the home directory",
            dir.display()
        )));
    }
    if !dir.is_dir() {
        return Err(AppError::Validation(format!(
            "'{}' is not a directory",
            dir.display()
        )));
    }

    let children: Vec<(PathBuf, bool)> = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| {
            // Symlinks are listed as plain entries and never entered.
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (e.path(), is_dir)
        })
        .collect();
    let paths: Vec<PathBuf> = children.iter().map(|(p, _)| p.clone()).collect();
    let sizes = sizing::sizes_of(&paths);

    let mut entries: Vec<AnalyzedEntry> = children
        .into_iter()
        .zip(sizes)
        .map(|((path, is_dir), bytes)| AnalyzedEntry {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
            is_dir,
            bytes,
        })
        .collect();
    entries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    let total_bytes = entries.iter().map(|e| e.bytes).sum();
    entries.truncate(MAX_ENTRIES);

    let parent = (dir != home)
        .then(|| dir.parent().map(|p| p.to_string_lossy().into_owned()))
        .flatten();
    Ok(AnalyzedDir {
        path: dir.to_string_lossy().into_owned(),
        parent,
        total_bytes,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![3u8; bytes]).unwrap();
    }

    #[test]
    fn lists_children_biggest_first() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join("small/a.bin"), 10_000);
        write(&home.path().join("big/b.bin"), 1_000_000);
        write(&home.path().join("loose.bin"), 100_000);
        let listing = analyze_dir(home.path(), None).unwrap();
        let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["big", "loose.bin", "small"]);
        assert!(listing.entries[0].is_dir);
        assert!(!listing.entries[1].is_dir);
        assert!(listing.parent.is_none());
        assert_eq!(
            listing.total_bytes,
            listing.entries.iter().map(|e| e.bytes).sum::<u64>()
        );
    }

    #[test]
    fn subdirectory_reports_its_parent() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join("d/x.bin"), 5_000);
        let sub = home.path().join("d");
        let listing = analyze_dir(home.path(), Some(sub.to_str().unwrap())).unwrap();
        let expected = dunce::canonicalize(home.path()).unwrap();
        assert_eq!(listing.parent.as_deref(), expected.to_str());
    }

    #[test]
    fn rejects_paths_outside_home_and_non_directories() {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        assert!(analyze_dir(home.path(), Some(outside.path().to_str().unwrap())).is_err());
        write(&home.path().join("file.bin"), 10);
        let file = home.path().join("file.bin");
        assert!(analyze_dir(home.path(), Some(file.to_str().unwrap())).is_err());
        assert!(analyze_dir(home.path(), Some("relative/path")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_home_is_not_followed() {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write(&outside.path().join("huge.bin"), 2_000_000);
        std::os::unix::fs::symlink(outside.path(), home.path().join("link")).unwrap();
        let listing = analyze_dir(home.path(), None).unwrap();
        let link = listing.entries.iter().find(|e| e.name == "link").unwrap();
        assert!(!link.is_dir);
        assert!(link.bytes < 100_000);
        // Asking to open the link path directly resolves outside home: refused.
        let linked = home.path().join("link");
        assert!(analyze_dir(home.path(), Some(linked.to_str().unwrap())).is_err());
    }
}
