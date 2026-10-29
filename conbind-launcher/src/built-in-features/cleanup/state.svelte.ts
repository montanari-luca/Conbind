import {
  cleanupClean,
  cleanupDiskSpace,
  cleanupScan,
  setFocusLock,
  type CleanupCategoryId,
  type CleanupCategoryResult,
  type CleanupDiskSpace,
  type CleanupGroup,
} from '../../lib/ipc/commands';
import { feedbackService } from '../../services/feedback/feedbackService.svelte';
import { formatBytes } from '../../services/action/actionService.svelte';
import { t } from '../../services/i18n';
import {
  CATEGORIES_BY_GROUP,
  applyCleanResults,
  flattenRows,
  selectedTotals,
  setCategorySelected,
  sortCategories,
  startsSelected,
} from './cleanupLogic';
import { fullDiskAccess } from './fullDiskAccess.svelte';

class CleanupState {
  group = $state<CleanupGroup>('general');
  categories = $state<CleanupCategoryResult[]>([]);
  scanning = $state(false);
  cleaning = $state(false);
  selected = $state<Set<number>>(new Set());
  expanded = $state<Set<CleanupCategoryId>>(new Set());
  cursor = $state(0);
  disk = $state<CleanupDiskSpace | null>(null);

  private scanId: number | null = null;
  /** Invalidates an in-flight scan when a newer one starts. */
  private generation = 0;

  rows = $derived(flattenRows(this.categories, this.expanded));
  totals = $derived(selectedTotals(this.categories, this.selected));

  async open(group: CleanupGroup): Promise<void> {
    this.group = group;
    void fullDiskAccess.check();
    await this.scan();
  }

  async scan(): Promise<void> {
    const generation = ++this.generation;
    this.categories = [];
    this.selected = new Set();
    this.expanded = new Set();
    this.cursor = 0;
    this.scanId = null;
    this.scanning = true;
    void cleanupDiskSpace().then((disk) => {
      if (generation === this.generation) this.disk = disk;
    });
    try {
      const scanId = await cleanupScan(CATEGORIES_BY_GROUP[this.group], (result) => {
        if (generation !== this.generation) return;
        this.categories = sortCategories([...this.categories, result]);
        // Caches are regenerable, so start with them ticked and let the user
        // untick; leftovers are a heuristic and are left for the user to opt in.
        if (!startsSelected(this.group)) return;
        const next = new Set(this.selected);
        for (const item of result.items) next.add(item.id);
        this.selected = next;
      });
      if (generation === this.generation) this.scanId = scanId;
    } finally {
      if (generation === this.generation) this.scanning = false;
    }
  }

  toggleItem(id: number): void {
    const next = new Set(this.selected);
    if (!next.delete(id)) next.add(id);
    this.selected = next;
  }

  toggleCategory(category: CleanupCategoryResult): void {
    const allOn = category.items.every((i) => this.selected.has(i.id));
    this.selected = setCategorySelected(category, this.selected, !allOn);
  }

  toggleExpanded(id: CleanupCategoryId): void {
    const next = new Set(this.expanded);
    if (!next.delete(id)) next.add(id);
    this.expanded = next;
  }

  moveCursor(delta: 1 | -1): void {
    const last = this.rows.length - 1;
    if (last < 0) return;
    this.cursor = Math.max(0, Math.min(last, this.cursor + delta));
  }

  /** Confirms with the user, then moves the selected items to the trash. */
  async requestClean(): Promise<void> {
    const { bytes, count } = this.totals;
    if (count === 0 || this.scanId === null || this.cleaning) return;

    await setFocusLock(true);
    let confirmed = false;
    try {
      confirmed = await feedbackService.confirmAlert({
        title: t('features.cleanup.confirm.title'),
        message: t('features.cleanup.confirm.message', { size: formatBytes(bytes), count }),
        confirmText: t('features.cleanup.confirm.confirm'),
        cancelText: t('features.cleanup.confirm.cancel'),
        variant: 'danger',
      });
    } finally {
      await setFocusLock(false);
    }
    if (!confirmed) return;

    this.cleaning = true;
    try {
      const results = await cleanupClean(this.scanId, [...this.selected]);
      const outcome = applyCleanResults(this.categories, results);
      this.categories = outcome.categories;
      this.selected = new Set(
        [...this.selected].filter((id) => !results.find((r) => r.id === id && r.ok)),
      );
      this.cursor = Math.min(this.cursor, Math.max(0, this.rows.length - 1));
      void cleanupDiskSpace().then((disk) => (this.disk = disk));
      await feedbackService.showHUD(
        outcome.failed > 0
          ? t('features.cleanup.hud.partial', {
              size: formatBytes(outcome.trashedBytes),
              failed: outcome.failed,
            })
          : t('features.cleanup.hud.done', { size: formatBytes(outcome.trashedBytes) }),
      );
    } finally {
      this.cleaning = false;
    }
  }

  reset(): void {
    this.generation++;
    this.categories = [];
    this.selected = new Set();
    this.expanded = new Set();
    this.cursor = 0;
    this.scanId = null;
    this.scanning = false;
    this.cleaning = false;
    this.disk = null;
  }
}

export const cleanupState = new CleanupState();
