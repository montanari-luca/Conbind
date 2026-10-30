import type { Extension, ExtensionContext, IFeedbackService } from 'conbind-sdk/contracts';
import {
  registerBuiltinSearchProvider,
  type BuiltinRowAction,
  type BuiltinSearchRow,
} from '../../services/search/builtinSearchProviders';
import { t } from '../../services/i18n';
import {
  mediaCommand,
  mediaNowPlaying,
  mediaSupported,
  type MediaCommand,
  type NowPlaying,
} from '../../lib/ipc/commands';

// Transport and now-playing both live in Rust (src-tauri/src/media). This
// extension maps commands onto them and renders the Now Playing row.

const COMMAND_IDS: Record<string, MediaCommand> = {
  'play-pause': 'playPause',
  'next-track': 'next',
  'previous-track': 'previous',
  stop: 'stop',
};

const HUD_LABELS: Record<MediaCommand, string> = {
  playPause: 'features.media.hud.play_pause',
  play: 'features.media.hud.play',
  pause: 'features.media.hud.pause',
  next: 'features.media.hud.next',
  previous: 'features.media.hud.previous',
  stop: 'features.media.hud.stop',
};

const NOW_PLAYING_ROW_ID = 'now-playing';
const NOW_PLAYING_QUERIES = ['now playing', 'playing', 'media', 'music', 'song', 'track', 'player'];

/** Search stays inside the provider budget; a slow read falls back to the last value. */
const READ_BUDGET_MS = 150;
const FRESH_MS = 1500;
const STALE_MS = 30_000;

/** The row shows for short prefixes of its keywords ("pla", "now pl"), never for one or two letters. */
export function matchesNowPlayingQuery(query: string): boolean {
  const q = query.trim().toLowerCase();
  return q.length >= 3 && NOW_PLAYING_QUERIES.some((keyword) => keyword.startsWith(q));
}

class MediaExtension implements Extension {
  private enabled = true;
  private supported = true;
  private feedbackService?: IFeedbackService;
  private cache?: { track: NowPlaying | null; at: number };
  private inflight?: Promise<NowPlaying | null>;

  onUnload = () => {};

  async initialize(context: ExtensionContext): Promise<void> {
    this.feedbackService = context.getService<IFeedbackService>('feedback');
  }

  async executeCommand(commandId: string): Promise<any> {
    const command = COMMAND_IDS[commandId];
    if (!command) return undefined;
    // A failure has already surfaced its own diagnostic; leave the launcher open
    // so it can be read.
    if (!(await this.run(command))) return undefined;
    await this.feedbackService?.showHUD(t(HUD_LABELS[command]));
    return { type: 'no-view' };
  }

  async activate(): Promise<void> {
    this.enabled = true;
    this.supported = await mediaSupported().catch(() => false);
    // Warm the backend (the macOS adapter probe is slow on first use) so the
    // first Now Playing search is not empty.
    if (this.supported) void this.refresh();
  }

  async deactivate(): Promise<void> {
    this.enabled = false;
    this.cache = undefined;
  }

  async searchRows(query: string): Promise<BuiltinSearchRow[]> {
    if (!this.enabled || !this.supported || !matchesNowPlayingQuery(query)) return [];
    const track = await this.currentTrack();
    if (!track) return [];
    return [
      {
        id: NOW_PLAYING_ROW_ID,
        score: 1.0,
        title: track.title,
        subtitle: [track.artist, track.appName].filter(Boolean).join(' · ') || undefined,
        icon: track.isPlaying ? 'icon:pause' : 'icon:play',
        priority: 'top',
        primaryActionLabel: t(
          track.isPlaying ? 'features.media.row.pause' : 'features.media.row.play',
        ),
      },
    ];
  }

  rowActions(rowId: string): BuiltinRowAction[] {
    if (rowId !== NOW_PLAYING_ROW_ID) return [];
    return [
      {
        id: 'media:next',
        label: t('features.media.row.next'),
        icon: 'icon:skip-forward',
        execute: async () => void (await this.run('next')),
      },
      {
        id: 'media:previous',
        label: t('features.media.row.previous'),
        icon: 'icon:skip-back',
        execute: async () => void (await this.run('previous')),
      },
    ];
  }

  async executeSearchResult(rowId: string): Promise<void> {
    if (rowId === NOW_PLAYING_ROW_ID) await this.run('playPause');
  }

  private async run(command: MediaCommand): Promise<boolean> {
    try {
      await mediaCommand(command);
      this.cache = undefined;
      return true;
    } catch {
      return false;
    }
  }

  private async currentTrack(): Promise<NowPlaying | null> {
    const now = Date.now();
    if (this.cache && now - this.cache.at < FRESH_MS) return this.cache.track;

    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const result = await Promise.race([
        this.refresh(),
        new Promise<'timeout'>((resolve) => {
          timer = setTimeout(() => resolve('timeout'), READ_BUDGET_MS);
        }),
      ]);
      if (result !== 'timeout') return result;
    } finally {
      if (timer) clearTimeout(timer);
    }
    return this.cache && now - this.cache.at < STALE_MS ? this.cache.track : null;
  }

  private refresh(): Promise<NowPlaying | null> {
    this.inflight ??= mediaNowPlaying()
      .catch(() => null)
      .then((track) => {
        this.cache = { track, at: Date.now() };
        return track;
      })
      .finally(() => {
        this.inflight = undefined;
      });
    return this.inflight;
  }
}

const extension = new MediaExtension();
registerBuiltinSearchProvider({
  extensionId: 'media',
  search: (query) => extension.searchRows(query),
  execute: (id) => extension.executeSearchResult(id),
  rowActions: (id) => extension.rowActions(id),
});
export default extension;
