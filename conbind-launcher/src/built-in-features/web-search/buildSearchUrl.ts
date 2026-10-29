import type { LauncherSearchEngine } from '../../services/settings/types/AppSettingsType';

export const DEFAULT_SEARCH_ENGINE: LauncherSearchEngine = 'google';

const ENGINE_URLS: Record<LauncherSearchEngine, string> = {
  google: 'https://www.google.com/search?q=',
  duckduckgo: 'https://duckduckgo.com/?q=',
  bing: 'https://www.bing.com/search?q=',
  brave: 'https://search.brave.com/search?q=',
};

export function isSearchEngine(value: unknown): value is LauncherSearchEngine {
  return typeof value === 'string' && Object.hasOwn(ENGINE_URLS, value);
}

/** The text after the `$` prefix, or `null` when `query` is not a web search. */
export function parseWebQuery(query: string): string | null {
  const trimmed = query.trim();
  if (!trimmed.startsWith('$')) return null;
  const text = trimmed.slice(1).trim();
  return text || null;
}

export function buildSearchUrl(engine: unknown, text: string): string {
  const base = isSearchEngine(engine) ? ENGINE_URLS[engine] : ENGINE_URLS[DEFAULT_SEARCH_ENGINE];
  return base + encodeURIComponent(text);
}
