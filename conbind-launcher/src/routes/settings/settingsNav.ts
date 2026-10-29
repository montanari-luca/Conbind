export type SettingsTabId =
  | 'general'
  | 'appearance'
  | 'extensions'
  | 'browsers'
  | 'applications'
  | 'file-search'
  | 'scripts'
  | 'ai'
  | 'backup'
  | 'privacy'
  | 'advanced'
  | 'developer'
  | 'about';

export interface SettingsNavItem {
  id: SettingsTabId;
  labelKey: string;
  subtitleKey: string;
  icon: string;
}

export interface SettingsNavGroup {
  id: string;
  titleKey: string;
  items: SettingsNavItem[];
}

const item = (id: SettingsTabId, tabKey: string, icon: string): SettingsNavItem => ({
  id,
  labelKey: `settings.tabs.${tabKey}`,
  subtitleKey: `settings.subtitles.${tabKey}`,
  icon,
});

export const SETTINGS_NAV_GROUPS: SettingsNavGroup[] = [
  {
    id: 'basics',
    titleKey: 'settings.groups.basics',
    items: [item('general', 'general', 'settings'), item('appearance', 'appearance', 'palette')],
  },
  {
    id: 'search',
    titleKey: 'settings.groups.search',
    items: [
      item('extensions', 'extensions', 'puzzle'),
      item('applications', 'applications', 'layers'),
      item('browsers', 'browsers', 'globe'),
      item('file-search', 'file_search', 'folder-search'),
      item('scripts', 'scripts', 'terminal'),
    ],
  },
  {
    id: 'data',
    titleKey: 'settings.groups.data',
    items: [
      item('ai', 'ai', 'ai-chat'),
      item('backup', 'backup', 'cloud-upload'),
      item('privacy', 'privacy', 'lock'),
    ],
  },
  {
    id: 'system',
    titleKey: 'settings.groups.system',
    items: [
      item('advanced', 'advanced', 'sliders'),
      item('developer', 'developer', 'dev-tools'),
      item('about', 'about', 'info'),
    ],
  },
];

/** The nav as shown to the user: the Developer page only exists while
 *  developer mode is on. */
export function visibleNavGroups(developerEnabled: boolean): SettingsNavGroup[] {
  return SETTINGS_NAV_GROUPS.map((group) => ({
    ...group,
    items: group.items.filter((entry) => entry.id !== 'developer' || developerEnabled),
  }));
}

export function findNavItem(id: string): SettingsNavItem | undefined {
  for (const group of SETTINGS_NAV_GROUPS) {
    const match = group.items.find((entry) => entry.id === id);
    if (match) return match;
  }
  return undefined;
}
