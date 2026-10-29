import type {
  CleanupCategoryId,
  CleanupCategoryResult,
  CleanupGroup,
  CleanupItem,
  CleanupItemResult,
} from '../../lib/ipc/commands';

/** Display order of categories, and which group each belongs to. */
export const CATEGORY_ORDER: CleanupCategoryId[] = [
  'userCaches',
  'logs',
  'tempFiles',
  'packageCaches',
  'xcodeDerivedData',
  'xcodeDeviceSupport',
  'appLeftovers',
];

export const CATEGORIES_BY_GROUP: Record<CleanupGroup, CleanupCategoryId[]> = {
  general: ['userCaches', 'logs', 'tempFiles'],
  developer: ['packageCaches', 'xcodeDerivedData', 'xcodeDeviceSupport'],
  leftovers: ['appLeftovers'],
};

/**
 * Groups whose findings are regenerable caches, so they start ticked. Leftovers
 * are a heuristic and start unticked: the user opts each one in.
 */
export function startsSelected(group: CleanupGroup): boolean {
  return group !== 'leftovers';
}

export type CleanupRow =
  | { kind: 'category'; category: CleanupCategoryResult }
  | { kind: 'item'; category: CleanupCategoryResult; item: CleanupItem };

export type SelectionState = 'all' | 'some' | 'none';

export function sortCategories(categories: CleanupCategoryResult[]): CleanupCategoryResult[] {
  const rank = (c: CleanupCategoryResult) => CATEGORY_ORDER.indexOf(c.category);
  return [...categories].sort((a, b) => rank(a) - rank(b));
}

/** Rows to render, in order: each non-empty category, then its items when expanded. */
export function flattenRows(
  categories: CleanupCategoryResult[],
  expanded: ReadonlySet<CleanupCategoryId>,
): CleanupRow[] {
  const rows: CleanupRow[] = [];
  for (const category of categories) {
    if (category.items.length === 0) continue;
    rows.push({ kind: 'category', category });
    if (expanded.has(category.category)) {
      for (const item of category.items) rows.push({ kind: 'item', category, item });
    }
  }
  return rows;
}

export function selectionState(
  category: CleanupCategoryResult,
  selected: ReadonlySet<number>,
): SelectionState {
  const picked = category.items.filter((i) => selected.has(i.id)).length;
  if (picked === 0) return 'none';
  return picked === category.items.length ? 'all' : 'some';
}

export function selectedTotals(
  categories: CleanupCategoryResult[],
  selected: ReadonlySet<number>,
): { bytes: number; count: number } {
  let bytes = 0;
  let count = 0;
  for (const category of categories) {
    for (const item of category.items) {
      if (selected.has(item.id)) {
        bytes += item.bytes;
        count += 1;
      }
    }
  }
  return { bytes, count };
}

/** Returns a new selection with every item of `category` set on or off. */
export function setCategorySelected(
  category: CleanupCategoryResult,
  selected: ReadonlySet<number>,
  on: boolean,
): Set<number> {
  const next = new Set(selected);
  for (const item of category.items) {
    if (on) next.add(item.id);
    else next.delete(item.id);
  }
  return next;
}

/**
 * Applies a clean's per-item results: drops the items that were trashed and
 * reports how many bytes that freed and how many failed.
 */
export function applyCleanResults(
  categories: CleanupCategoryResult[],
  results: CleanupItemResult[],
): { categories: CleanupCategoryResult[]; trashedBytes: number; failed: number } {
  const ok = new Set(results.filter((r) => r.ok).map((r) => r.id));
  let trashedBytes = 0;
  const remaining = categories.map((category) => {
    const items = category.items.filter((item) => {
      if (!ok.has(item.id)) return true;
      trashedBytes += item.bytes;
      return false;
    });
    return { ...category, items, bytes: items.reduce((sum, i) => sum + i.bytes, 0) };
  });
  return {
    categories: remaining,
    trashedBytes,
    failed: results.filter((r) => !r.ok).length,
  };
}
