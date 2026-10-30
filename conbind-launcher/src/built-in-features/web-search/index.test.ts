import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../../services/log/logService', () => ({
  logService: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));
vi.mock('../../services/opener/openerService', () => ({
  openerService: { open: vi.fn() },
}));
vi.mock('../../services/settings/settingsService.svelte', () => ({
  settingsService: { currentSettings: { search: {} } },
}));
vi.mock('../../lib/ipc/commands', () => ({ hideWindow: vi.fn() }));
vi.mock('../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: { report: vi.fn(), showHUD: vi.fn() },
}));

import './index';
import {
  executeBuiltinSearchResult,
  searchBuiltinProviders,
} from '../../services/search/builtinSearchProviders';
import { openerService } from '../../services/opener/openerService';
import { settingsService } from '../../services/settings/settingsService.svelte';
import { hideWindow } from '../../lib/ipc/commands';

const ROW_ID = 'web_search_open';

describe('web-search built-in', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (settingsService.currentSettings as { search: object }).search = {};
  });

  it('returns one top row for a $ query', async () => {
    const hits = await searchBuiltinProviders('$Learn java', () => true);
    const mine = hits.filter((h) => h.extensionId === 'web-search');
    expect(mine).toHaveLength(1);
    expect(mine[0]).toMatchObject({
      id: ROW_ID,
      icon: 'icon:globe',
      priority: 'top',
      actionPayload: { text: 'Learn java' },
    });
  });

  it('returns nothing without the prefix or text', async () => {
    for (const q of ['Learn java', '$', '$  ']) {
      const hits = await searchBuiltinProviders(q, () => true);
      expect(hits.filter((h) => h.extensionId === 'web-search')).toEqual([]);
    }
  });

  it('is skipped when the built-in is disabled', async () => {
    const hits = await searchBuiltinProviders('$java', (id) => id !== 'web-search');
    expect(hits.filter((h) => h.extensionId === 'web-search')).toEqual([]);
  });

  it('opens a Google search by default and hides the launcher', async () => {
    await executeBuiltinSearchResult('web-search', ROW_ID, { text: 'Learn java' });
    expect(openerService.open).toHaveBeenCalledWith(
      null,
      'https://www.google.com/search?q=Learn%20java',
    );
    expect(hideWindow).toHaveBeenCalled();
  });

  it('uses the engine chosen in settings', async () => {
    (settingsService.currentSettings as { search: object }).search = {
      webSearchEngine: 'duckduckgo',
    };
    await executeBuiltinSearchResult('web-search', ROW_ID, { text: 'x' });
    expect(openerService.open).toHaveBeenCalledWith(null, 'https://duckduckgo.com/?q=x');
  });
});
