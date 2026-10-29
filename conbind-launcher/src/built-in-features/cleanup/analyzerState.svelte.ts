import {
  cleanupAnalyzeDir,
  setFocusLock,
  showInFileManager,
  trashPath,
  type AnalyzedDir,
  type AnalyzedEntry,
} from '../../lib/ipc/commands';
import { feedbackService } from '../../services/feedback/feedbackService.svelte';
import { formatBytes } from '../../services/action/actionService.svelte';
import { t } from '../../services/i18n';
import { removeFromListings } from './analyzerLogic';
import { fullDiskAccess } from './fullDiskAccess.svelte';

class AnalyzerState {
  /** Listings loaded so far, root first. Going back up reuses them. */
  stack = $state<AnalyzedDir[]>([]);
  /** Cursor position to restore in each listing of `stack`. */
  private cursors: number[] = [];
  cursor = $state(0);
  loading = $state(false);

  /** Invalidates an in-flight load when the user navigates again. */
  private generation = 0;

  current = $derived(this.stack.length ? this.stack[this.stack.length - 1] : null);
  selected = $derived(this.current?.entries[this.cursor] ?? null);

  async open(): Promise<void> {
    this.stack = [];
    this.cursors = [];
    this.cursor = 0;
    void fullDiskAccess.check();
    await this.load(undefined, false);
  }

  private async load(path: string | undefined, push: boolean): Promise<void> {
    const generation = ++this.generation;
    this.loading = true;
    try {
      const dir = await cleanupAnalyzeDir(path);
      if (generation !== this.generation) return;
      if (push) this.cursors[this.stack.length - 1] = this.cursor;
      this.stack = push ? [...this.stack, dir] : [dir];
      this.cursors[this.stack.length - 1] = 0;
      this.cursor = 0;
    } finally {
      if (generation === this.generation) this.loading = false;
    }
  }

  async enter(entry: AnalyzedEntry | null = this.selected): Promise<void> {
    if (!entry?.isDir || this.loading) return;
    await this.load(entry.path, true);
  }

  up(): void {
    if (this.stack.length <= 1) return;
    this.generation++;
    this.loading = false;
    this.stack = this.stack.slice(0, -1);
    this.cursor = this.cursors[this.stack.length - 1] ?? 0;
  }

  moveCursor(delta: 1 | -1): void {
    const last = (this.current?.entries.length ?? 0) - 1;
    if (last < 0) return;
    this.cursor = Math.max(0, Math.min(last, this.cursor + delta));
  }

  async reveal(entry: AnalyzedEntry | null = this.selected): Promise<void> {
    if (entry) await showInFileManager(entry.path);
  }

  /** Confirms, then moves one entry to the trash and updates the sizes shown. */
  async trash(entry: AnalyzedEntry | null = this.selected): Promise<void> {
    if (!entry) return;
    await setFocusLock(true);
    let confirmed = false;
    try {
      confirmed = await feedbackService.confirmAlert({
        title: t('features.cleanup.entry_confirm.title', { name: entry.name }),
        message: t('features.cleanup.entry_confirm.message', { size: formatBytes(entry.bytes) }),
        confirmText: t('features.cleanup.confirm.confirm'),
        cancelText: t('features.cleanup.confirm.cancel'),
        variant: 'danger',
      });
    } finally {
      await setFocusLock(false);
    }
    if (!confirmed) return;

    await trashPath(entry.path);
    this.stack = removeFromListings(this.stack, entry.path, entry.bytes);
    this.cursor = Math.min(this.cursor, Math.max(0, (this.current?.entries.length ?? 1) - 1));
    await feedbackService.showHUD(
      t('features.cleanup.hud.done', { size: formatBytes(entry.bytes) }),
    );
  }

  reset(): void {
    this.generation++;
    this.stack = [];
    this.cursors = [];
    this.cursor = 0;
    this.loading = false;
  }
}

export const analyzerState = new AnalyzerState();
