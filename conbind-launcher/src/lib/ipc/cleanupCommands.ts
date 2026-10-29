import { Channel } from '@tauri-apps/api/core';
import { invokeSafe } from './invokeSafe';

// ── Cleanup (caches, temp files, developer leftovers) ─────────────────────────

export type CleanupCategoryId =
  | 'userCaches'
  | 'logs'
  | 'tempFiles'
  | 'xcodeDerivedData'
  | 'xcodeDeviceSupport'
  | 'packageCaches'
  | 'appLeftovers';

export type CleanupGroup = 'general' | 'developer' | 'leftovers';

export interface CleanupItem {
  id: number;
  label: string;
  path: string;
  bytes: number;
}

export interface CleanupCategoryResult {
  category: CleanupCategoryId;
  group: CleanupGroup;
  items: CleanupItem[];
  bytes: number;
}

export interface CleanupItemResult {
  id: number;
  ok: boolean;
  error: string | null;
}

export interface CleanupDiskSpace {
  freeBytes: number;
  totalBytes: number;
}

/**
 * Measures the given categories. `onCategory` fires as each one finishes so the
 * view can fill in sizes progressively; the resolved value is the scan id to
 * pass to {@link cleanupClean}.
 */
export async function cleanupScan(
  categories: CleanupCategoryId[],
  onCategory: (result: CleanupCategoryResult) => void,
): Promise<number> {
  const onEvent = new Channel<CleanupCategoryResult>();
  onEvent.onmessage = onCategory;
  return invokeSafe<number>('cleanup_scan', { categories, onEvent });
}

/** Moves the chosen items of a scan to the trash. */
export async function cleanupClean(
  scanId: number,
  itemIds: number[],
): Promise<CleanupItemResult[]> {
  return invokeSafe<CleanupItemResult[]>('cleanup_clean', { scanId, itemIds });
}

export async function cleanupDiskSpace(): Promise<CleanupDiskSpace | null> {
  return invokeSafe<CleanupDiskSpace | null>('cleanup_disk_space', undefined, { silent: true });
}

// ── Space analyzer ────────────────────────────────────────────────────────────

export interface AnalyzedEntry {
  name: string;
  path: string;
  isDir: boolean;
  bytes: number;
}

export interface AnalyzedDir {
  path: string;
  /** `null` at the home directory: the analyzer never goes above it. */
  parent: string | null;
  totalBytes: number;
  entries: AnalyzedEntry[];
}

/** One directory level of the home folder (or `path`), biggest entries first. */
export async function cleanupAnalyzeDir(path?: string): Promise<AnalyzedDir> {
  return invokeSafe<AnalyzedDir>('cleanup_analyze_dir', { path: path ?? null });
}

// ── Maintenance actions ───────────────────────────────────────────────────────

export type NetworkCleanupOutcome = 'done' | 'cancelled';

/** Flushes DNS (and ARP on macOS). `cancelled` means the OS password prompt was dismissed. */
export async function cleanupNetwork(): Promise<NetworkCleanupOutcome> {
  return invokeSafe<NetworkCleanupOutcome>('cleanup_network');
}

/** Permanently empties the OS trash. Confirm with the user first. */
export async function cleanupEmptyTrash(): Promise<void> {
  await invokeSafe('cleanup_empty_trash');
}

// ── Full Disk Access (macOS) ──────────────────────────────────────────────────

/** Whether protected folders are readable. Always `true` off macOS. */
export async function cleanupFullDiskAccess(): Promise<boolean> {
  return invokeSafe<boolean>('cleanup_full_disk_access', undefined, { silent: true });
}

/** Opens System Settings at Full Disk Access. Resolves `false` if it couldn't. */
export async function cleanupOpenFullDiskAccess(): Promise<boolean> {
  return invokeSafe<boolean>('cleanup_open_full_disk_access');
}
