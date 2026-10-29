import type { AnalyzedDir } from '../../lib/ipc/commands';

/** Whether `path` is `ancestor` itself or inside it, for either path separator. */
function isWithin(path: string, ancestor: string): boolean {
  if (path === ancestor) return true;
  const sep = ancestor.includes('\\') ? '\\' : '/';
  const prefix = ancestor.endsWith(sep) ? ancestor : ancestor + sep;
  return path.startsWith(prefix);
}

/**
 * Updates every loaded listing after `path` (worth `bytes`) was trashed: the
 * entry disappears from its own listing, and each ancestor shrinks by the same
 * amount, so going back up shows correct sizes without rescanning the disk.
 */
export function removeFromListings(
  listings: AnalyzedDir[],
  path: string,
  bytes: number,
): AnalyzedDir[] {
  return listings.map((dir) => {
    if (!isWithin(path, dir.path) || path === dir.path) return dir;
    const entries = dir.entries
      .filter((entry) => entry.path !== path)
      .map((entry) =>
        isWithin(path, entry.path) ? { ...entry, bytes: Math.max(0, entry.bytes - bytes) } : entry,
      );
    // A direct child may be absent from a truncated listing; the total still
    // includes it, so shrink the total either way.
    return { ...dir, entries, totalBytes: Math.max(0, dir.totalBytes - bytes) };
  });
}

/** Meter fill for a row, relative to the largest entry in the listing. */
export function entryFraction(dir: AnalyzedDir, bytes: number): number {
  const largest = dir.entries[0]?.bytes ?? 0;
  return largest > 0 ? bytes / largest : 0;
}
