//! Disk-usage measurement for the cleanup scanner and space analyzer.
//!
//! Deliberately separate from the file index walker: that one skips caches,
//! `node_modules` and `Library` and honours `.gitignore`, which are exactly the
//! places cleanup has to measure. This walker follows nothing and skips
//! nothing, and reports allocated bytes (what `du` shows) rather than apparent
//! size, so a sparse VM image doesn't read as freeable space.

use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const MAX_WORKERS: usize = 8;

#[cfg(unix)]
fn entry_size(md: &Metadata, seen: &mut HashSet<(u64, u64)>) -> u64 {
    use std::os::unix::fs::MetadataExt;
    // A hard-linked file is one allocation however many names it has, so
    // count each (device, inode) once per measurement.
    if md.nlink() > 1 && !md.is_dir() && !seen.insert((md.dev(), md.ino())) {
        return 0;
    }
    md.blocks() * 512
}

#[cfg(not(unix))]
fn entry_size(md: &Metadata, _seen: &mut HashSet<(u64, u64)>) -> u64 {
    md.len()
}

/// Allocated bytes under `path`. Symlinks are measured as links and never
/// followed; unreadable directories count as empty.
pub fn size_of(path: &Path) -> u64 {
    let Ok(root_md) = fs::symlink_metadata(path) else {
        return 0;
    };
    let mut seen = HashSet::new();
    let mut total = entry_size(&root_md, &mut seen);
    if !root_md.is_dir() {
        return total;
    }
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            // `DirEntry::metadata` does not traverse symlinks.
            let Ok(md) = entry.metadata() else { continue };
            total += entry_size(&md, &mut seen);
            if md.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    total
}

/// Sizes of several paths, measured in parallel. Results keep input order.
pub fn sizes_of(paths: &[PathBuf]) -> Vec<u64> {
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(MAX_WORKERS)
        .min(paths.len());
    if workers <= 1 {
        return paths.iter().map(|p| size_of(p)).collect();
    }
    let next = AtomicUsize::new(0);
    let results = Mutex::new(vec![0u64; paths.len()]);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(i) else { break };
                let size = size_of(path);
                if let Ok(mut out) = results.lock() {
                    out[i] = size;
                }
            });
        }
    });
    results.into_inner().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7u8; bytes]).unwrap();
    }

    #[test]
    fn missing_path_is_zero() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(size_of(&dir.path().join("nope")), 0);
    }

    #[test]
    fn counts_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a/one.bin"), 100_000);
        write(&dir.path().join("a/b/two.bin"), 200_000);
        // Allocated size rounds up to whole blocks, so it is at least the
        // data written.
        assert!(size_of(dir.path()) >= 300_000);
    }

    #[test]
    fn larger_tree_measures_larger() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("small/x.bin"), 10_000);
        write(&dir.path().join("big/y.bin"), 1_000_000);
        let sizes = sizes_of(&[dir.path().join("small"), dir.path().join("big")]);
        assert!(sizes[1] > sizes[0]);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        write(&target.join("huge.bin"), 2_000_000);
        let linked = dir.path().join("linked");
        fs::create_dir(&linked).unwrap();
        std::os::unix::fs::symlink(&target, linked.join("link")).unwrap();
        assert!(size_of(&linked) < 100_000);
    }

    #[cfg(unix)]
    #[test]
    fn matches_du_on_a_real_tree() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a/one.bin"), 123_457);
        write(&dir.path().join("a/b/two.bin"), 2_000_001);
        write(&dir.path().join("a/b/c/three.bin"), 7);
        fs::hard_link(
            dir.path().join("a/one.bin"),
            dir.path().join("a/b/one-link.bin"),
        )
        .unwrap();
        std::os::unix::fs::symlink(dir.path().join("a"), dir.path().join("a/loop")).unwrap();

        let out = std::process::Command::new("du")
            .arg("-sk")
            .arg(dir.path())
            .output()
            .expect("run du");
        let du_kib: u64 = String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .expect("parse du output");
        assert_eq!(size_of(dir.path()) / 1024, du_kib);
    }

    #[cfg(unix)]
    #[test]
    fn hard_links_count_once() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("d/a.bin");
        write(&a, 1_000_000);
        fs::hard_link(&a, dir.path().join("d/b.bin")).unwrap();
        let once = size_of(&a);
        assert!(size_of(&dir.path().join("d")) < once * 2);
    }
}
