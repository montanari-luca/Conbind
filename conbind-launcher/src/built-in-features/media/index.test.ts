import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { ExtensionContext } from 'conbind-sdk/contracts';

vi.mock('../../lib/ipc/commands', () => ({
  mediaCommand: vi.fn(),
  mediaNowPlaying: vi.fn(),
  mediaSupported: vi.fn(),
}));
vi.mock('../../services/i18n', () => ({ t: (key: string) => key }));

import { mediaCommand, mediaNowPlaying, mediaSupported } from '../../lib/ipc/commands';
import { searchBuiltinProviders } from '../../services/search/builtinSearchProviders';
import mediaExtension, { matchesNowPlayingQuery } from './index';

const showHUD = vi.fn();
const context = {
  getService: () => ({ showHUD }),
} as unknown as ExtensionContext;

const track = {
  title: 'Song',
  artist: 'Artist',
  album: null,
  appName: 'Spotify',
  isPlaying: true,
};

describe('MediaExtension commands', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    vi.mocked(mediaSupported).mockResolvedValue(true);
    vi.mocked(mediaNowPlaying).mockResolvedValue(null);
    await mediaExtension.initialize(context);
    await mediaExtension.activate();
  });

  it.each([
    ['play-pause', 'playPause', 'features.media.hud.play_pause'],
    ['next-track', 'next', 'features.media.hud.next'],
    ['previous-track', 'previous', 'features.media.hud.previous'],
    ['stop', 'stop', 'features.media.hud.stop'],
  ])('maps %s to the %s media command and confirms with a HUD', async (id, command, hud) => {
    vi.mocked(mediaCommand).mockResolvedValueOnce(true);

    const result = await mediaExtension.executeCommand(id);

    expect(mediaCommand).toHaveBeenCalledWith(command);
    expect(showHUD).toHaveBeenCalledWith(hud);
    expect(result).toEqual({ type: 'no-view' });
  });

  it('stays open and shows no HUD when the player rejects the command', async () => {
    vi.mocked(mediaCommand).mockRejectedValueOnce(new Error('no player'));

    const result = await mediaExtension.executeCommand('next-track');

    expect(showHUD).not.toHaveBeenCalled();
    expect(result).toBeUndefined();
  });

  it('ignores unknown commands', async () => {
    expect(await mediaExtension.executeCommand('unknown')).toBeUndefined();
    expect(mediaCommand).not.toHaveBeenCalled();
  });
});

describe('Now Playing row', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.mocked(mediaSupported).mockResolvedValue(true);
    vi.mocked(mediaNowPlaying).mockResolvedValue(null);
    await mediaExtension.initialize(context);
    await mediaExtension.activate();
    // `activate` warms the cache in the background; let that settle, then move
    // past its freshness window so each test reads the player it just mocked.
    await new Promise((resolve) => setTimeout(resolve, 0));
    vi.setSystemTime(Date.now() + 5_000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('matches keyword prefixes but not short or unrelated queries', () => {
    expect(matchesNowPlayingQuery('now playing')).toBe(true);
    expect(matchesNowPlayingQuery('Pla')).toBe(true);
    expect(matchesNowPlayingQuery('music')).toBe(true);
    expect(matchesNowPlayingQuery('pl')).toBe(false);
    expect(matchesNowPlayingQuery('calendar')).toBe(false);
  });

  it('shows the current track with its artist and app', async () => {
    vi.mocked(mediaNowPlaying).mockResolvedValue(track);

    const rows = await mediaExtension.searchRows('now playing');

    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      id: 'now-playing',
      title: 'Song',
      subtitle: 'Artist · Spotify',
      icon: 'icon:pause',
      primaryActionLabel: 'features.media.row.pause',
    });
  });

  it('offers play for a paused track and omits missing metadata from the subtitle', async () => {
    vi.mocked(mediaNowPlaying).mockResolvedValue({
      ...track,
      artist: null,
      appName: null,
      isPlaying: false,
    });

    const [row] = await mediaExtension.searchRows('media');

    expect(row.subtitle).toBeUndefined();
    expect(row.icon).toBe('icon:play');
    expect(row.primaryActionLabel).toBe('features.media.row.play');
  });

  it('returns no row when nothing is playing', async () => {
    expect(await mediaExtension.searchRows('now playing')).toEqual([]);
  });

  it('returns no row for unrelated queries without reading the player', async () => {
    vi.mocked(mediaNowPlaying).mockClear();
    expect(await mediaExtension.searchRows('safari')).toEqual([]);
    expect(mediaNowPlaying).not.toHaveBeenCalled();
  });

  it('returns no row when the platform does not support media', async () => {
    vi.mocked(mediaSupported).mockResolvedValue(false);
    await mediaExtension.activate();
    vi.mocked(mediaNowPlaying).mockResolvedValue(track);

    expect(await mediaExtension.searchRows('now playing')).toEqual([]);
  });

  it('is gated on the extension being active', async () => {
    vi.mocked(mediaNowPlaying).mockResolvedValue(track);
    await mediaExtension.deactivate();

    expect(await mediaExtension.searchRows('now playing')).toEqual([]);
  });

  it('plays or pauses when the row is chosen', async () => {
    vi.mocked(mediaCommand).mockResolvedValue(true);
    await mediaExtension.executeSearchResult('now-playing');
    expect(mediaCommand).toHaveBeenCalledWith('playPause');
  });

  it('adds Next and Previous as row actions only for its own row', async () => {
    vi.mocked(mediaCommand).mockResolvedValue(true);
    expect(mediaExtension.rowActions('other')).toEqual([]);

    const [next, previous] = mediaExtension.rowActions('now-playing');
    await next.execute();
    await previous.execute();

    expect(vi.mocked(mediaCommand).mock.calls).toEqual([['next'], ['previous']]);
  });

  it('is registered as a built-in search provider', async () => {
    vi.mocked(mediaNowPlaying).mockResolvedValue(track);

    const hits = await searchBuiltinProviders('now playing', (id) => id === 'media');

    expect(hits.map((h) => h.extensionId)).toEqual(['media']);
  });
});
