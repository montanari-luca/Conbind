//! What the cleanup scanner looks at, per OS.
//!
//! Everything here is user-scope: a target is only used if it resolves under
//! the home directory, which is also the only place `validate_trash_path`
//! lets the cleaner act. On macOS and Linux `$TMPDIR` / `/tmp` sit outside home,
//! so the temp category is Windows-only in practice.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    UserCaches,
    Logs,
    TempFiles,
    XcodeDerivedData,
    XcodeDeviceSupport,
    PackageCaches,
    AppLeftovers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    General,
    Developer,
    Leftovers,
}

impl Category {
    pub const ALL: [Category; 7] = [
        Category::UserCaches,
        Category::Logs,
        Category::TempFiles,
        Category::XcodeDerivedData,
        Category::XcodeDeviceSupport,
        Category::PackageCaches,
        Category::AppLeftovers,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Category::UserCaches => "userCaches",
            Category::Logs => "logs",
            Category::TempFiles => "tempFiles",
            Category::XcodeDerivedData => "xcodeDerivedData",
            Category::XcodeDeviceSupport => "xcodeDeviceSupport",
            Category::PackageCaches => "packageCaches",
            Category::AppLeftovers => "appLeftovers",
        }
    }

    pub fn from_id(id: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.id() == id)
    }

    pub fn group(self) -> Group {
        match self {
            Category::UserCaches | Category::Logs | Category::TempFiles => Group::General,
            Category::XcodeDerivedData | Category::XcodeDeviceSupport | Category::PackageCaches => {
                Group::Developer
            }
            Category::AppLeftovers => Group::Leftovers,
        }
    }
}

/// One place the scanner measures. `per_child` makes each direct child its own
/// item (so the user sees which app or project owns the bytes); otherwise the
/// path itself is the single item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub category: Category,
    pub path: PathBuf,
    pub per_child: bool,
}

fn target(category: Category, path: PathBuf, per_child: bool) -> Target {
    Target {
        category,
        path,
        per_child,
    }
}

/// All targets for the current OS that exist under `home`. `temp_dir` is passed
/// in so tests don't depend on the machine's environment.
pub fn targets(home: &Path, temp_dir: &Path) -> Vec<Target> {
    let mut out = Vec::new();

    if cfg!(target_os = "macos") {
        let lib = home.join("Library");
        out.push(target(Category::UserCaches, lib.join("Caches"), true));
        out.push(target(Category::Logs, lib.join("Logs"), true));
        let xcode = lib.join("Developer/Xcode");
        out.push(target(
            Category::XcodeDerivedData,
            xcode.join("DerivedData"),
            true,
        ));
        out.push(target(
            Category::XcodeDeviceSupport,
            xcode.join("iOS DeviceSupport"),
            true,
        ));
        out.push(target(
            Category::PackageCaches,
            lib.join("pnpm/store"),
            false,
        ));
    } else if cfg!(target_os = "windows") {
        let local = home.join("AppData/Local");
        out.push(target(
            Category::PackageCaches,
            local.join("npm-cache"),
            false,
        ));
        out.push(target(
            Category::PackageCaches,
            local.join("pnpm/store"),
            false,
        ));
    } else {
        out.push(target(Category::UserCaches, home.join(".cache"), true));
        out.push(target(
            Category::PackageCaches,
            home.join(".local/share/pnpm/store"),
            false,
        ));
    }

    out.push(target(
        Category::PackageCaches,
        home.join(".npm/_cacache"),
        false,
    ));
    out.push(target(
        Category::PackageCaches,
        home.join(".cargo/registry/cache"),
        false,
    ));
    out.push(target(
        Category::PackageCaches,
        home.join(".gradle/caches"),
        false,
    ));

    if temp_dir.starts_with(home) {
        out.push(target(Category::TempFiles, temp_dir.to_path_buf(), true));
    }

    out.retain(|t| t.path.is_dir());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_ids_round_trip() {
        for c in Category::ALL {
            assert_eq!(Category::from_id(c.id()), Some(c));
        }
        assert_eq!(Category::from_id("nope"), None);
    }

    #[test]
    fn only_existing_directories_are_targets() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".npm/_cacache")).unwrap();
        let found = targets(home.path(), Path::new("/definitely/not/under/home"));
        assert!(found
            .iter()
            .any(|t| t.category == Category::PackageCaches && t.path.ends_with("_cacache")));
        assert!(found.iter().all(|t| t.path.is_dir()));
    }

    #[test]
    fn temp_dir_outside_home_is_ignored() {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let found = targets(home.path(), outside.path());
        assert!(found.iter().all(|t| t.category != Category::TempFiles));
    }

    #[test]
    fn temp_dir_inside_home_is_a_per_child_target() {
        let home = tempfile::tempdir().unwrap();
        let temp = home.path().join("tmp");
        std::fs::create_dir_all(&temp).unwrap();
        let found = targets(home.path(), &temp);
        let t = found
            .iter()
            .find(|t| t.category == Category::TempFiles)
            .expect("temp target");
        assert!(t.per_child);
    }
}
