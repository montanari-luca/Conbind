// conbind-launcher/src/lib/ipc/mediaCommands.ts
// Tauri command wrappers, re-exported through ./commands (the barrel).
import { invokeSafe, invokeSafeVoid } from './invokeSafe';
import type { MediaCommand, NowPlaying } from '../../bindings';

// ── Media controls ────────────────────────────────────────────────────────────

/** Generated from Rust's `MediaCommand` / `NowPlaying` — see `../../bindings`. */
export type { MediaCommand, NowPlaying };

/** Whether this machine can attempt media transport at all. */
export async function mediaSupported(): Promise<boolean> {
  return invokeSafe<boolean>('media_supported', undefined, { silent: true });
}

/** Rejects with `IpcError` (diagnostic already reported) when no player accepts the command. */
export async function mediaCommand(command: MediaCommand): Promise<boolean> {
  return invokeSafeVoid('media_command', { command });
}

/** `null` when nothing is playing or readable. Silent: this backs a passive search row. */
export async function mediaNowPlaying(): Promise<NowPlaying | null> {
  return invokeSafe<NowPlaying | null>('media_now_playing', undefined, { silent: true });
}
