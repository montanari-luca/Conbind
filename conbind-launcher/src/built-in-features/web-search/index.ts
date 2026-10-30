import {
  registerBuiltinSearchProvider,
  type BuiltinSearchRow,
} from '../../services/search/builtinSearchProviders';
import { openerService } from '../../services/opener/openerService';
import { settingsService } from '../../services/settings/settingsService.svelte';
import { feedbackService } from '../../services/feedback/feedbackService.svelte';
import { hideWindow } from '../../lib/ipc/commands';
import { t } from '../../services/i18n';
import type { Extension, ExtensionContext } from 'conbind-sdk/contracts';
import { buildSearchUrl, parseWebQuery } from './buildSearchUrl';

const SEARCH_ROW_ID = 'web_search_open';

class WebSearchExtension implements Extension {
  private enabled = true;

  async initialize(_context: ExtensionContext): Promise<void> {}

  async activate(): Promise<void> {
    this.enabled = true;
  }

  async deactivate(): Promise<void> {
    this.enabled = false;
  }

  async executeCommand(): Promise<unknown> {
    return undefined;
  }

  async searchRows(query: string): Promise<BuiltinSearchRow[]> {
    if (!this.enabled) return [];
    const text = parseWebQuery(query);
    if (!text) return [];
    return [
      {
        id: SEARCH_ROW_ID,
        score: 1.0,
        title: t('features.web_search.search_for', { query: text }),
        icon: 'icon:globe',
        priority: 'top',
        primaryActionLabel: t('features.web_search.search'),
        actionPayload: { text },
      },
    ];
  }

  async executeSearchResult(_rowId: string, payload?: unknown): Promise<void> {
    const text = (payload as { text?: unknown } | undefined)?.text;
    if (typeof text !== 'string' || !text) return;
    const url = buildSearchUrl(settingsService.currentSettings.search?.webSearchEngine, text);
    try {
      await openerService.open(null, url);
      await hideWindow();
    } catch (err) {
      void feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: { message: t('features.web_search.open_failed') },
        developerDetail: String(err),
      });
    }
  }
}

const extension = new WebSearchExtension();
registerBuiltinSearchProvider({
  extensionId: 'web-search',
  search: (query) => extension.searchRows(query),
  execute: (rowId, payload) => extension.executeSearchResult(rowId, payload),
});
export default extension;
