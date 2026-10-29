import { describe, expect, it } from 'vitest';
import type { CleanupCategoryResult } from '../../lib/ipc/commands';
import {
  CATEGORIES_BY_GROUP,
  CATEGORY_ORDER,
  applyCleanResults,
  flattenRows,
  selectedTotals,
  selectionState,
  setCategorySelected,
  sortCategories,
  startsSelected,
} from './cleanupLogic';

function category(
  id: CleanupCategoryResult['category'],
  items: Array<[number, number]>,
): CleanupCategoryResult {
  return {
    category: id,
    group: 'general',
    items: items.map(([itemId, bytes]) => ({
      id: itemId,
      label: `item-${itemId}`,
      path: `/home/u/${itemId}`,
      bytes,
    })),
    bytes: items.reduce((sum, [, b]) => sum + b, 0),
  };
}

describe('cleanupLogic', () => {
  const caches = category('userCaches', [
    [1, 300],
    [2, 100],
  ]);
  const logs = category('logs', [[3, 50]]);
  const empty = category('tempFiles', []);

  it('sorts categories into display order', () => {
    expect(sortCategories([logs, caches]).map((c) => c.category)).toEqual(['userCaches', 'logs']);
  });

  it('flattens collapsed categories to one row each and skips empty ones', () => {
    const rows = flattenRows([caches, logs, empty], new Set());
    expect(rows.map((r) => r.kind)).toEqual(['category', 'category']);
  });

  it('lists items under an expanded category', () => {
    const rows = flattenRows([caches, logs], new Set(['userCaches']));
    expect(rows.map((r) => r.kind)).toEqual(['category', 'item', 'item', 'category']);
  });

  it('reports all, some and none selection', () => {
    expect(selectionState(caches, new Set([1, 2]))).toBe('all');
    expect(selectionState(caches, new Set([2]))).toBe('some');
    expect(selectionState(caches, new Set())).toBe('none');
  });

  it('totals only the selected items', () => {
    expect(selectedTotals([caches, logs], new Set([1, 3]))).toEqual({ bytes: 350, count: 2 });
  });

  it('toggles a whole category without mutating the input', () => {
    const start = new Set([3]);
    const on = setCategorySelected(caches, start, true);
    expect([...on].sort()).toEqual([1, 2, 3]);
    expect([...start]).toEqual([3]);
    expect([...setCategorySelected(caches, on, false)]).toEqual([3]);
  });

  it('removes trashed items and counts failures', () => {
    const { categories, trashedBytes, failed } = applyCleanResults(
      [caches, logs],
      [
        { id: 1, ok: true, error: null },
        { id: 3, ok: false, error: 'denied' },
      ],
    );
    expect(trashedBytes).toBe(300);
    expect(failed).toBe(1);
    expect(categories[0].items.map((i) => i.id)).toEqual([2]);
    expect(categories[0].bytes).toBe(100);
    expect(categories[1].items.map((i) => i.id)).toEqual([3]);
  });

  it('gives every category exactly one group and a display position', () => {
    const grouped = Object.values(CATEGORIES_BY_GROUP).flat();
    expect([...grouped].sort()).toEqual([...CATEGORY_ORDER].sort());
  });

  it('only pre-ticks regenerable caches, never leftovers', () => {
    expect(startsSelected('general')).toBe(true);
    expect(startsSelected('developer')).toBe(true);
    expect(startsSelected('leftovers')).toBe(false);
  });
});
