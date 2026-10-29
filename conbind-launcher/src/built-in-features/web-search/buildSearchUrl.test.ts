import { describe, it, expect } from 'vitest';
import { buildSearchUrl, parseWebQuery } from './buildSearchUrl';

describe('parseWebQuery', () => {
  it('reads the text after $ with or without a space', () => {
    expect(parseWebQuery('$Learn java')).toBe('Learn java');
    expect(parseWebQuery('$ Learn java')).toBe('Learn java');
    expect(parseWebQuery('  $  rust  ')).toBe('rust');
  });

  it('ignores queries without the prefix or without text', () => {
    expect(parseWebQuery('Learn java')).toBeNull();
    expect(parseWebQuery('$')).toBeNull();
    expect(parseWebQuery('$   ')).toBeNull();
    expect(parseWebQuery('')).toBeNull();
  });
});

describe('buildSearchUrl', () => {
  it('encodes the query for the chosen engine', () => {
    expect(buildSearchUrl('duckduckgo', 'a&b c')).toBe('https://duckduckgo.com/?q=a%26b%20c');
    expect(buildSearchUrl('bing', 'x')).toBe('https://www.bing.com/search?q=x');
    expect(buildSearchUrl('brave', 'x')).toBe('https://search.brave.com/search?q=x');
  });

  it('falls back to Google for a missing or unknown engine', () => {
    expect(buildSearchUrl(undefined, 'java')).toBe('https://www.google.com/search?q=java');
    expect(buildSearchUrl('altavista', 'java')).toBe('https://www.google.com/search?q=java');
  });
});
