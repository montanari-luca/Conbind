//! Leftover data from apps that are no longer installed (macOS).
//!
//! This is a heuristic, so it is deliberately conservative and its results are
//! shown for review rather than pre-selected. An entry only counts as a
//! leftover when all of these hold:
//! - its name is a reverse-DNS bundle id with at least three segments,
//! - it is not Apple's (`com.apple.*`), a shared `group.*` container, or ours,
//! - no installed app's bundle id matches it or is a dotted prefix of it
//!   (so helpers like `com.vendor.app.helper` stay with `com.vendor.app`),
//! - it has not been touched in the last week, which covers an app that is
//!   in use from somewhere the installed-app scan doesn't look.

use crate::application::service::extract_bundle_id;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MIN_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// `(folder under home, suffix stripped from the name to get the bundle id)`.
const LOCATIONS: &[(&str, &str)] = &[
    ("Library/Application Support", ""),
    ("Library/Caches", ""),
    ("Library/Containers", ""),
    ("Library/HTTPStorages", ""),
    ("Library/WebKit", ""),
    ("Library/Saved Application State", ".savedState"),
    ("Library/Preferences", ".plist"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftover {
    pub path: PathBuf,
    /// Canonical folder the entry must stay inside (e.g. `~/Library/Caches`).
    pub root: PathBuf,
}

fn looks_like_bundle_id(id: &str) -> bool {
    let segments: Vec<&str> = id.split('.').collect();
    segments.len() >= 3
        && segments.iter().all(|s| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

fn belongs_to_installed(id: &str, installed: &[String]) -> bool {
    installed.iter().any(|app| {
        id == app
            || id
                .strip_prefix(app.as_str())
                .is_some_and(|r| r.starts_with('.'))
            || app.strip_prefix(id).is_some_and(|r| r.starts_with('.'))
    })
}

fn is_protected(id: &str) -> bool {
    id.starts_with("com.apple.") || id.starts_with("group.")
}

/// Leftovers under `home`, given the bundle ids of every installed app.
pub fn find_leftovers(home: &Path, installed: &[String], now: SystemTime) -> Vec<Leftover> {
    let mut out = Vec::new();
    for (sub, suffix) in LOCATIONS {
        let Ok(root) = dunce::canonicalize(home.join(sub)) else {
            continue;
        };
        let Ok(read) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let id = if suffix.is_empty() {
                name.as_str()
            } else {
                match name.strip_suffix(suffix) {
                    Some(id) => id,
                    None => continue,
                }
            };
            if !looks_like_bundle_id(id) || is_protected(id) || belongs_to_installed(id, installed)
            {
                continue;
            }
            // A symlink may point into shared or cloud-linked storage.
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            let recent = meta
                .modified()
                .ok()
                .and_then(|m| now.duration_since(m).ok())
                .is_none_or(|age| age < MIN_AGE);
            if recent {
                continue;
            }
            out.push(Leftover {
                path: root.join(&name),
                root: root.clone(),
            });
        }
    }
    out
}

/// Bundle ids of the `.app` bundles directly inside `dirs`, looking one folder
/// deeper too (e.g. `/Applications/Utilities`, `/Applications/Setapp`).
pub fn installed_bundle_ids(dirs: &[PathBuf]) -> Vec<String> {
    fn walk(dir: &Path, depth: u8, out: &mut Vec<String>) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("app") {
                out.extend(extract_bundle_id(&path));
            } else if depth > 0 && path.is_dir() {
                walk(&path, depth - 1, out);
            }
        }
    }
    let mut out = Vec::new();
    for dir in dirs {
        walk(dir, 1, &mut out);
    }
    out
}

/// Where macOS apps normally live.
pub fn default_app_dirs(home: &Path) -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        home.join("Applications"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn old() -> SystemTime {
        // "Now" is far past anything created in a test, so everything is old.
        SystemTime::now() + Duration::from_secs(30 * 24 * 60 * 60)
    }

    fn mk(home: &Path, rel: &str) {
        let p = home.join(rel);
        if rel.ends_with(".plist") {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, b"x").unwrap();
        } else {
            fs::create_dir_all(p.join("data")).unwrap();
        }
    }

    fn found(home: &Path, installed: &[&str], now: SystemTime) -> Vec<String> {
        let installed: Vec<String> = installed.iter().map(|s| s.to_string()).collect();
        let mut names: Vec<String> = find_leftovers(home, &installed, now)
            .iter()
            .map(|l| l.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn flags_data_of_apps_that_are_not_installed() {
        let home = tempfile::tempdir().unwrap();
        mk(home.path(), "Library/Caches/com.gone.app");
        mk(home.path(), "Library/Application Support/com.gone.app");
        mk(home.path(), "Library/Preferences/com.gone.app.plist");
        mk(
            home.path(),
            "Library/Saved Application State/com.gone.app.savedState",
        );
        assert_eq!(
            found(home.path(), &[], old()),
            [
                "com.gone.app",
                "com.gone.app",
                "com.gone.app.plist",
                "com.gone.app.savedState"
            ]
        );
    }

    #[test]
    fn keeps_data_of_installed_apps_and_their_helpers() {
        let home = tempfile::tempdir().unwrap();
        mk(home.path(), "Library/Caches/com.keep.app");
        mk(home.path(), "Library/Caches/com.keep.app.helper");
        mk(home.path(), "Library/Preferences/com.keep.app.plist");
        assert!(found(home.path(), &["com.keep.app"], old()).is_empty());
    }

    #[test]
    fn never_flags_apple_group_or_non_bundle_names() {
        let home = tempfile::tempdir().unwrap();
        mk(home.path(), "Library/Caches/com.apple.Safari");
        mk(home.path(), "Library/Containers/group.com.vendor.shared");
        mk(home.path(), "Library/Application Support/Spotify");
        mk(home.path(), "Library/Application Support/com.vendor");
        mk(home.path(), "Library/Caches/not a bundle.id.here");
        assert!(found(home.path(), &[], old()).is_empty());
    }

    #[test]
    fn skips_recently_modified_entries() {
        let home = tempfile::tempdir().unwrap();
        mk(home.path(), "Library/Caches/com.gone.app");
        assert!(found(home.path(), &[], SystemTime::now()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinks() {
        let home = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        fs::create_dir_all(home.path().join("Library/Caches")).unwrap();
        std::os::unix::fs::symlink(
            elsewhere.path(),
            home.path().join("Library/Caches/com.gone.app"),
        )
        .unwrap();
        assert!(found(home.path(), &[], old()).is_empty());
    }

    #[test]
    fn leftover_stays_inside_its_canonical_root() {
        let home = tempfile::tempdir().unwrap();
        mk(home.path(), "Library/Caches/com.gone.app");
        let l = &find_leftovers(home.path(), &[], old())[0];
        assert!(l.path.starts_with(&l.root));
        assert_eq!(
            l.root,
            dunce::canonicalize(home.path().join("Library/Caches")).unwrap()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_bundle_ids_from_app_bundles() {
        let dir = tempfile::tempdir().unwrap();
        let contents = dir.path().join("Sub/Thing.app/Contents");
        fs::create_dir_all(&contents).unwrap();
        let mut dict = plist::Dictionary::new();
        dict.insert("CFBundleIdentifier".into(), "com.example.thing".into());
        plist::Value::Dictionary(dict)
            .to_file_xml(contents.join("Info.plist"))
            .unwrap();
        assert_eq!(
            installed_bundle_ids(&[dir.path().to_path_buf()]),
            ["com.example.thing"]
        );
    }
}
