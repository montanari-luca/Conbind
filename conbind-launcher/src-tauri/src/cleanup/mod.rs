//! Cache, temp and developer-leftover cleanup.
//!
//! The scanner measures a fixed set of per-OS targets (see [`targets`]) and
//! records every item it finds in a [`ScanRegistry`]. The cleaner only accepts
//! item ids from a recorded scan, never raw paths, and re-validates each item
//! against its own target root immediately before moving it to the OS trash.
//! Nothing here deletes permanently.

pub mod analyzer;
pub mod network;
pub mod remnants;
pub mod sizing;
pub mod targets;
pub mod trash_bin;

use crate::commands::file_manager::validate_trash_path;
use crate::error::AppError;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;
use targets::{Category, Group, Target};

/// Scans kept for cleaning. Older ones are evicted so a long session doesn't
/// accumulate item tables.
const MAX_SCANS: usize = 4;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedItem {
    pub id: u32,
    pub label: String,
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryResult {
    pub category: Category,
    pub group: Group,
    pub items: Vec<ScannedItem>,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub id: u32,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Default)]
struct ScanRecord {
    next_item: u32,
    /// item id -> (item path, canonical root it must stay under)
    items: HashMap<u32, (PathBuf, PathBuf)>,
}

#[derive(Default)]
struct RegistryInner {
    next_scan: u64,
    scans: VecDeque<(u64, ScanRecord)>,
}

/// Tauri-managed state: the item tables of recent scans.
#[derive(Default)]
pub struct ScanRegistry {
    inner: Mutex<RegistryInner>,
}

impl ScanRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&self) -> Result<u64, AppError> {
        let mut inner = self.inner.lock().map_err(|_| AppError::Lock)?;
        inner.next_scan += 1;
        let id = inner.next_scan;
        inner.scans.push_back((id, ScanRecord::default()));
        while inner.scans.len() > MAX_SCANS {
            inner.scans.pop_front();
        }
        Ok(id)
    }

    /// Registers an item and returns its id. Fails if the scan was evicted.
    fn add(&self, scan_id: u64, path: PathBuf, root: PathBuf) -> Result<u32, AppError> {
        let mut inner = self.inner.lock().map_err(|_| AppError::Lock)?;
        let record = inner
            .scans
            .iter_mut()
            .find(|(id, _)| *id == scan_id)
            .map(|(_, r)| r)
            .ok_or_else(|| AppError::NotFound(format!("cleanup scan {scan_id}")))?;
        record.next_item += 1;
        let id = record.next_item;
        record.items.insert(id, (path, root));
        Ok(id)
    }

    fn lookup(&self, scan_id: u64, item_id: u32) -> Result<Option<(PathBuf, PathBuf)>, AppError> {
        let inner = self.inner.lock().map_err(|_| AppError::Lock)?;
        let record = inner
            .scans
            .iter()
            .find(|(id, _)| *id == scan_id)
            .map(|(_, r)| r)
            .ok_or_else(|| AppError::NotFound(format!("cleanup scan {scan_id}")))?;
        Ok(record.items.get(&item_id).cloned())
    }

    /// Measures every requested category's targets, recording items in the
    /// scan and handing each finished category to `on_category` so the UI can
    /// fill in sizes as they arrive.
    pub fn scan(
        &self,
        scan_id: u64,
        home: &Path,
        temp_dir: &Path,
        categories: &[Category],
        mut on_category: impl FnMut(CategoryResult),
    ) -> Result<(), AppError> {
        let all = targets::targets(home, temp_dir);
        for category in categories {
            // Leftovers are found by `scan_leftovers`, not from a fixed target.
            if *category == Category::AppLeftovers {
                continue;
            }
            let mut items = Vec::new();
            for target in all.iter().filter(|t| t.category == *category) {
                self.scan_target(scan_id, target, &mut items)?;
            }
            let bytes = items.iter().map(|i| i.bytes).sum();
            items.sort_by(|a, b| b.bytes.cmp(&a.bytes));
            on_category(CategoryResult {
                category: *category,
                group: category.group(),
                items,
                bytes,
            });
        }
        Ok(())
    }

    /// Records the leftovers found by [`remnants::find_leftovers`] as a
    /// category result, sized and sorted like any other.
    pub fn scan_leftovers(
        &self,
        scan_id: u64,
        leftovers: Vec<remnants::Leftover>,
    ) -> Result<CategoryResult, AppError> {
        let paths: Vec<PathBuf> = leftovers.iter().map(|l| l.path.clone()).collect();
        let sizes = sizing::sizes_of(&paths);
        let mut items = Vec::new();
        for (leftover, bytes) in leftovers.into_iter().zip(sizes) {
            if bytes == 0 {
                continue;
            }
            let label = leftover
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let id = self.add(scan_id, leftover.path.clone(), leftover.root)?;
            items.push(ScannedItem {
                id,
                label,
                path: leftover.path.to_string_lossy().into_owned(),
                bytes,
            });
        }
        items.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        let bytes = items.iter().map(|i| i.bytes).sum();
        Ok(CategoryResult {
            category: Category::AppLeftovers,
            group: Group::Leftovers,
            items,
            bytes,
        })
    }

    fn scan_target(
        &self,
        scan_id: u64,
        target: &Target,
        out: &mut Vec<ScannedItem>,
    ) -> Result<(), AppError> {
        let Ok(root) = dunce::canonicalize(&target.path) else {
            return Ok(());
        };
        let paths: Vec<PathBuf> = if target.per_child {
            std::fs::read_dir(&root)
                .map(|rd| rd.flatten().map(|e| e.path()).collect())
                .unwrap_or_default()
        } else {
            vec![root.clone()]
        };
        let sizes = sizing::sizes_of(&paths);
        for (path, bytes) in paths.into_iter().zip(sizes) {
            if bytes == 0 {
                continue;
            }
            let label = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let id = self.add(scan_id, path.clone(), root.clone())?;
            out.push(ScannedItem {
                id,
                label,
                path: path.to_string_lossy().into_owned(),
                bytes,
            });
        }
        Ok(())
    }

    /// Moves the chosen items of a recorded scan to the trash. Each item is
    /// re-validated first; a failure on one never stops the rest.
    pub fn clean(
        &self,
        scan_id: u64,
        item_ids: &[u32],
        home: &Path,
        trash: impl Fn(&Path) -> Result<(), String>,
    ) -> Result<Vec<ItemResult>, AppError> {
        let mut results = Vec::with_capacity(item_ids.len());
        for &id in item_ids {
            let outcome = match self.lookup(scan_id, id)? {
                None => Err(format!("unknown cleanup item {id}")),
                Some((path, root)) => trash_item(&path, &root, home, &trash),
            };
            results.push(match outcome {
                Ok(()) => ItemResult {
                    id,
                    ok: true,
                    error: None,
                },
                Err(error) => ItemResult {
                    id,
                    ok: false,
                    error: Some(error),
                },
            });
        }
        Ok(results)
    }
}

fn trash_item(
    path: &Path,
    root: &Path,
    home: &Path,
    trash: &impl Fn(&Path) -> Result<(), String>,
) -> Result<(), String> {
    // Already gone (e.g. the app cleaned its own cache since the scan).
    if std::fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    let target = validate_trash_path(&path.to_string_lossy(), home).map_err(|e| e.to_string())?;
    if !target.starts_with(root) {
        return Err("path is no longer inside its scanned folder".to_string());
    }
    trash(&target)
}

/// Runs a platform tool to completion, mapping a failure to spawn into an error.
pub(crate) fn run_command(mut command: Command) -> Result<Output, AppError> {
    command
        .output()
        .map_err(|e| AppError::Platform(format!("could not run {:?}: {e}", command.get_program())))
}

/// Whether Conbind can read macOS's TCC-protected folders (Full Disk Access).
/// Without it the analyzer silently under-counts `~/Library`. Other platforms
/// have no equivalent gate, so they always report `true`.
pub fn full_disk_access_granted(home: &Path) -> bool {
    cfg!(not(target_os = "macos")) || probe_readable(&home.join("Library/Safari"))
}

/// `false` only when the folder exists but reading it is denied. A missing
/// folder proves nothing, so it doesn't count against the user.
fn probe_readable(dir: &Path) -> bool {
    match std::fs::read_dir(dir) {
        Ok(_) => true,
        Err(e) => e.kind() != std::io::ErrorKind::PermissionDenied,
    }
}

/// Free and total space on the volume that holds `home`.
pub fn disk_space(home: &Path) -> Option<DiskSpace> {
    let home = dunce::canonicalize(home).ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|d| home.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| DiskSpace {
            free_bytes: d.available_space(),
            total_bytes: d.total_space(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let cache = home.path().join(".npm/_cacache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("blob"), vec![1u8; 200_000]).unwrap();
        (home, cache)
    }

    fn scan_one(reg: &ScanRegistry, home: &Path) -> (u64, Vec<CategoryResult>) {
        let id = reg.begin().unwrap();
        let mut results = Vec::new();
        reg.scan(
            id,
            home,
            Path::new("/not/under/home"),
            &[Category::PackageCaches],
            |r| results.push(r),
        )
        .unwrap();
        (id, results)
    }

    #[test]
    fn scan_reports_items_with_sizes() {
        let (home, _) = fixture();
        let reg = ScanRegistry::new();
        let (_, results) = scan_one(&reg, home.path());
        assert_eq!(results.len(), 1);
        assert!(results[0].bytes >= 200_000);
        assert_eq!(results[0].items.len(), 1);
    }

    #[test]
    fn clean_trashes_scanned_items() {
        let (home, cache) = fixture();
        let reg = ScanRegistry::new();
        let (scan, results) = scan_one(&reg, home.path());
        let id = results[0].items[0].id;
        let trashed = RefCell::new(Vec::new());
        let out = reg
            .clean(scan, &[id], home.path(), |p| {
                trashed.borrow_mut().push(p.to_path_buf());
                Ok(())
            })
            .unwrap();
        assert!(out[0].ok);
        assert_eq!(trashed.borrow()[0], dunce::canonicalize(&cache).unwrap());
    }

    #[test]
    fn clean_rejects_unknown_item_and_unknown_scan() {
        let (home, _) = fixture();
        let reg = ScanRegistry::new();
        let (scan, _) = scan_one(&reg, home.path());
        let out = reg
            .clean(scan, &[9999], home.path(), |_| panic!("must not trash"))
            .unwrap();
        assert!(!out[0].ok);
        assert!(reg
            .clean(scan + 100, &[1], home.path(), |_| panic!("must not trash"))
            .is_err());
    }

    #[test]
    fn clean_refuses_a_path_swapped_for_an_outside_symlink() {
        let (home, cache) = fixture();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("keep"), b"x").unwrap();
        let reg = ScanRegistry::new();
        let (scan, results) = scan_one(&reg, home.path());
        let id = results[0].items[0].id;
        // After the scan, the cache dir is replaced by a link to elsewhere.
        fs::remove_dir_all(&cache).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &cache).unwrap();
        let out = reg
            .clean(scan, &[id], home.path(), |p| {
                // A symlink is trashed as a link, never its target.
                assert!(fs::symlink_metadata(p).unwrap().file_type().is_symlink());
                Ok(())
            })
            .unwrap();
        assert!(out[0].ok);
        assert!(outside.path().join("keep").exists());
    }

    #[test]
    fn missing_item_is_treated_as_already_cleaned() {
        let (home, cache) = fixture();
        let reg = ScanRegistry::new();
        let (scan, results) = scan_one(&reg, home.path());
        let id = results[0].items[0].id;
        fs::remove_dir_all(&cache).unwrap();
        let out = reg
            .clean(scan, &[id], home.path(), |_| panic!("must not trash"))
            .unwrap();
        assert!(out[0].ok);
    }

    #[test]
    fn probe_treats_missing_and_readable_dirs_as_fine() {
        let dir = tempfile::tempdir().unwrap();
        assert!(probe_readable(dir.path()));
        assert!(probe_readable(&dir.path().join("missing")));
    }

    #[cfg(unix)]
    #[test]
    fn probe_reports_a_permission_denied_dir() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let locked = dir.path().join("locked");
        fs::create_dir(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = probe_readable(&locked);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        // Root can read anything, so only assert when the OS enforced it.
        if fs::read_dir(&locked).is_ok() && readable {
            return;
        }
        assert!(!readable);
    }

    #[test]
    fn leftovers_are_registered_and_cleanable_within_their_root() {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join("Library/Caches/com.gone.app");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("blob"), vec![1u8; 100_000]).unwrap();
        let reg = ScanRegistry::new();
        let scan = reg.begin().unwrap();
        let far_future = std::time::SystemTime::now() + std::time::Duration::from_secs(30 * 86_400);
        let found = remnants::find_leftovers(home.path(), &[], far_future);
        let result = reg.scan_leftovers(scan, found).unwrap();
        assert_eq!(result.category, Category::AppLeftovers);
        assert_eq!(result.items.len(), 1);
        let trashed = RefCell::new(0);
        let out = reg
            .clean(scan, &[result.items[0].id], home.path(), |_| {
                *trashed.borrow_mut() += 1;
                Ok(())
            })
            .unwrap();
        assert!(out[0].ok);
        assert_eq!(*trashed.borrow(), 1);
    }

    #[test]
    fn registry_evicts_old_scans() {
        let reg = ScanRegistry::new();
        let first = reg.begin().unwrap();
        for _ in 0..MAX_SCANS {
            reg.begin().unwrap();
        }
        assert!(reg
            .add(first, PathBuf::from("/x"), PathBuf::from("/"))
            .is_err());
    }
}
