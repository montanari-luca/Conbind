import { SETTINGS_NAV_GROUPS, type SettingsTabId } from './settingsNav';

/**
 * The searchable surface of the settings window. Each entry points at a row
 * that is always mounted on its page and reuses that row's own label key, so
 * the index cannot show text the page does not. `keywords` are untranslated
 * product and technology names a user might type.
 *
 * Page-level entries are derived from the nav, so every page is findable even
 * where its rows are not individually listed (Extensions, Applications, AI).
 */
export interface SettingsIndexEntry {
  tab: SettingsTabId;
  labelKey: string;
  descriptionKey?: string;
  keywords?: string[];
}

export const SETTINGS_INDEX: SettingsIndexEntry[] = [
  { tab: 'general', labelKey: 'settings.general.autostart', keywords: ['startup', 'login'] },
  { tab: 'general', labelKey: 'settings.general.hotkey', keywords: ['shortcut', 'keybinding'] },
  { tab: 'general', labelKey: 'settings.general.show_tray_icon', keywords: ['menu bar', 'tray'] },
  { tab: 'general', labelKey: 'settings.general.show_dock_icon', keywords: ['dock'] },
  { tab: 'general', labelKey: 'settings.general.onboarding', keywords: ['tour', 'welcome'] },

  {
    tab: 'appearance',
    labelKey: 'settings.appearance.theme',
    descriptionKey: 'settings.general.theme_description',
    keywords: ['dark', 'light', 'mode'],
  },
  {
    tab: 'appearance',
    labelKey: 'settings.appearance.palette',
    descriptionKey: 'settings.appearance.palette_description',
    keywords: ['blue mint', 'classic', 'color', 'accent'],
  },
  { tab: 'appearance', labelKey: 'settings.appearance.window_mode', keywords: ['compact'] },
  {
    tab: 'appearance',
    labelKey: 'settings.appearance.interface_size_label',
    keywords: ['scale', 'zoom', 'text size'],
  },
  { tab: 'appearance', labelKey: 'settings.general.display', keywords: ['monitor', 'screen'] },
  {
    tab: 'appearance',
    labelKey: 'settings.general.vertical_position',
    keywords: ['placement', 'top', 'center'],
  },
  { tab: 'appearance', labelKey: 'settings.general.snap_dragging', keywords: ['drag'] },

  { tab: 'file-search', labelKey: 'settings.file_search.file_search_title', keywords: ['index'] },
  { tab: 'file-search', labelKey: 'settings.file_search.root_search_title' },

  { tab: 'backup', labelKey: 'settings.backup.export_backup', keywords: ['import', 'restore'] },
  { tab: 'backup', labelKey: 'settings.backup.migrate_raycast', keywords: ['raycast'] },

  { tab: 'privacy', labelKey: 'settings.privacy.clipboard_privacy', keywords: ['history'] },
  { tab: 'privacy', labelKey: 'settings.privacy.retention_period', keywords: ['clipboard'] },
  { tab: 'privacy', labelKey: 'settings.privacy.encryption_at_rest', keywords: ['encrypt'] },

  { tab: 'advanced', labelKey: 'settings.advanced.extension_search' },
  { tab: 'advanced', labelKey: 'settings.advanced.auto_update_extensions' },
  { tab: 'advanced', labelKey: 'settings.advanced.pop_to_root', keywords: ['reset', 'idle'] },
  { tab: 'advanced', labelKey: 'settings.advanced.escape_key', keywords: ['esc'] },
  { tab: 'advanced', labelKey: 'settings.advanced.text_expansion', keywords: ['snippets'] },
  { tab: 'advanced', labelKey: 'settings.advanced.developer_mode', keywords: ['developer'] },

  { tab: 'developer', labelKey: 'settings.developer.devex_inspector', keywords: ['inspector'] },
  { tab: 'developer', labelKey: 'settings.developer.sideload', keywords: ['dev extension'] },
  { tab: 'developer', labelKey: 'settings.developer.tracing', keywords: ['ipc', 'rpc'] },
  { tab: 'developer', labelKey: 'settings.developer.verbose_logging', keywords: ['logs', 'debug'] },
  { tab: 'developer', labelKey: 'settings.developer.copy_diagnostics' },

  { tab: 'about', labelKey: 'settings.about.auto_updates', keywords: ['update', 'version'] },
  { tab: 'about', labelKey: 'settings.about.project_links', keywords: ['github', 'docs'] },
];

/** Search keywords for pages whose rows are not listed individually. */
const PAGE_KEYWORDS: Partial<Record<SettingsTabId, string[]>> = {
  ai: ['openai', 'anthropic', 'claude', 'gemini', 'ollama', 'openrouter', 'api key', 'model'],
  extensions: ['store', 'install', 'uninstall', 'plugin'],
  applications: ['apps', 'scan'],
  browsers: ['chrome', 'firefox', 'safari', 'bookmarks', 'profile'],
  scripts: ['shell', 'directory'],
};

export interface SettingsSearchHit {
  tab: SettingsTabId;
  /** The row to scroll to; absent for a hit on the page itself. */
  rowLabel?: string;
  label: string;
  description: string;
  /** The page the hit lives on, for the secondary line in the result list. */
  pageLabel: string;
}

const fold = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

/**
 * Case- and diacritic-insensitive match over translated label, description,
 * keywords and page name. Every whitespace-separated term must match somewhere.
 * Label matches rank above description/keyword matches, and a prefix match
 * ranks above a substring match.
 */
export function searchSettings(
  query: string,
  translate: (key: string) => string,
  visibleTabs: ReadonlySet<SettingsTabId>,
): SettingsSearchHit[] {
  const terms = fold(query).split(/\s+/).filter(Boolean);
  if (terms.length === 0) return [];

  const pages = SETTINGS_NAV_GROUPS.flatMap((group) => group.items).filter((entry) =>
    visibleTabs.has(entry.id),
  );
  const pageLabel = (tab: SettingsTabId) =>
    translate(pages.find((entry) => entry.id === tab)?.labelKey ?? '');

  const candidates: Array<SettingsSearchHit & { haystack: string; primary: string }> = [];

  for (const page of pages) {
    const label = translate(page.labelKey);
    const description = translate(page.subtitleKey);
    candidates.push({
      tab: page.id,
      label,
      description,
      pageLabel: label,
      primary: fold(label),
      haystack: fold([label, description, ...(PAGE_KEYWORDS[page.id] ?? [])].join(' ')),
    });
  }

  for (const entry of SETTINGS_INDEX) {
    if (!visibleTabs.has(entry.tab)) continue;
    const label = translate(entry.labelKey);
    const description = entry.descriptionKey ? translate(entry.descriptionKey) : '';
    candidates.push({
      tab: entry.tab,
      rowLabel: label,
      label,
      description,
      pageLabel: pageLabel(entry.tab),
      primary: fold(label),
      haystack: fold([label, description, ...(entry.keywords ?? [])].join(' ')),
    });
  }

  const scored: Array<{ hit: SettingsSearchHit; score: number }> = [];
  for (const candidate of candidates) {
    if (!terms.every((term) => candidate.haystack.includes(term))) continue;
    const inLabel = terms.every((term) => candidate.primary.includes(term));
    const prefix = candidate.primary.startsWith(terms[0]);
    const score = (inLabel ? 2 : 0) + (prefix ? 1 : 0) + (candidate.rowLabel ? 0 : 0.5);
    const { haystack: _h, primary: _p, ...hit } = candidate;
    scored.push({ hit, score });
  }

  return scored.sort((a, b) => b.score - a.score).map((s) => s.hit);
}
