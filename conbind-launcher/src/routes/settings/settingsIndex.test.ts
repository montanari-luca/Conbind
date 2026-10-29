import { readFileSync } from 'fs';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import { describe, expect, it } from 'vitest';
import { SETTINGS_INDEX, searchSettings } from './settingsIndex';
import {
  SETTINGS_NAV_GROUPS,
  findNavItem,
  visibleNavGroups,
  type SettingsTabId,
} from './settingsNav';

const en = JSON.parse(
  readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), '../../locales/en.json'), 'utf8'),
);

function lookup(key: string): string | undefined {
  let node: unknown = en;
  for (const part of key.split('.')) {
    if (typeof node !== 'object' || node === null || !(part in node)) return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === 'string' ? node : undefined;
}

const translate = (key: string) => lookup(key) ?? key;
const allTabs = new Set<SettingsTabId>(
  SETTINGS_NAV_GROUPS.flatMap((group) => group.items.map((entry) => entry.id)),
);
const withoutDeveloper = new Set(allTabs);
withoutDeveloper.delete('developer');

describe('settings nav', () => {
  it('has unique tab ids', () => {
    const ids = SETTINGS_NAV_GROUPS.flatMap((group) => group.items.map((entry) => entry.id));
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('resolves every label, subtitle and group title in the English catalog', () => {
    for (const group of SETTINGS_NAV_GROUPS) {
      expect(lookup(group.titleKey), group.titleKey).toBeTruthy();
      for (const entry of group.items) {
        expect(lookup(entry.labelKey), entry.labelKey).toBeTruthy();
        expect(lookup(entry.subtitleKey), entry.subtitleKey).toBeTruthy();
      }
    }
  });

  it('hides the Developer page unless developer mode is on', () => {
    const ids = (dev: boolean) =>
      visibleNavGroups(dev).flatMap((group) => group.items.map((entry) => entry.id));
    expect(ids(false)).not.toContain('developer');
    expect(ids(true)).toContain('developer');
  });

  it('finds items by id', () => {
    expect(findNavItem('appearance')?.labelKey).toBe('settings.tabs.appearance');
    expect(findNavItem('nope')).toBeUndefined();
  });
});

describe('settings index', () => {
  it('only points at pages that exist', () => {
    for (const entry of SETTINGS_INDEX) expect(allTabs.has(entry.tab), entry.labelKey).toBe(true);
  });

  it('only uses keys present in the English catalog', () => {
    for (const entry of SETTINGS_INDEX) {
      expect(lookup(entry.labelKey), entry.labelKey).toBeTruthy();
      if (entry.descriptionKey)
        expect(lookup(entry.descriptionKey), entry.descriptionKey).toBeTruthy();
    }
  });

  it('lists each row once per page, since rows are located by their label', () => {
    const seen = new Set<string>();
    for (const entry of SETTINGS_INDEX) {
      const id = `${entry.tab}:${translate(entry.labelKey)}`;
      expect(seen.has(id), id).toBe(false);
      seen.add(id);
    }
  });
});

describe('searchSettings', () => {
  it('returns nothing for an empty query', () => {
    expect(searchSettings('  ', translate, allTabs)).toEqual([]);
  });

  it('finds a row by label and points at its page and row', () => {
    const [hit] = searchSettings('launch at login', translate, allTabs);
    expect(hit).toMatchObject({ tab: 'general', rowLabel: 'Launch at Login' });
  });

  it('finds a row by keyword', () => {
    const hits = searchSettings('dock', translate, allTabs);
    expect(hits.some((h) => h.rowLabel === 'Show icon in Dock')).toBe(true);
  });

  it('is case- and diacritic-insensitive', () => {
    const plain = searchSettings(
      'cafe',
      (k) => (k === 'settings.tabs.general' ? 'Café' : translate(k)),
      allTabs,
    );
    expect(plain.some((h) => h.tab === 'general' && !h.rowLabel)).toBe(true);
    expect(searchSettings('PALETTE', translate, allTabs).length).toBeGreaterThan(0);
  });

  it('requires every term to match', () => {
    expect(searchSettings('palette zzzz', translate, allTabs)).toEqual([]);
    expect(searchSettings('color palette', translate, allTabs)[0]).toMatchObject({
      tab: 'appearance',
    });
  });

  it('finds pages that have no listed rows through their keywords', () => {
    const hits = searchSettings('openai', translate, allTabs);
    expect(hits[0].tab).toBe('ai');
    expect(hits[0].rowLabel).toBeUndefined();
  });

  it('ranks label matches above description-only matches', () => {
    const hits = searchSettings('theme', translate, allTabs);
    expect(hits[0].label.toLowerCase()).toContain('theme');
  });

  it('leaves out pages that are not visible', () => {
    const hits = searchSettings('verbose', translate, withoutDeveloper);
    expect(hits.some((h) => h.tab === 'developer')).toBe(false);
    expect(searchSettings('verbose', translate, allTabs).some((h) => h.tab === 'developer')).toBe(
      true,
    );
  });
});
