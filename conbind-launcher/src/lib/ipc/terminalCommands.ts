import { Channel } from '@tauri-apps/api/core';
import { invokeSafe, invokeSafeVoid } from './invokeSafe';

// ── Terminal (the launcher's `>` prefix) ──────────────────────────────────────

export type TerminalEvent =
  | { type: 'chunk'; stream: 'stdout' | 'stderr'; data: string }
  | { type: 'done'; exitCode: number | null; cwd: string; cancelled: boolean };

/**
 * Runs a shell command in the app-lifetime session. `onEvent` receives each
 * output line, then a final `done`; the promise settles after that.
 */
export async function terminalRun(
  command: string,
  onEvent: (event: TerminalEvent) => void,
): Promise<void> {
  const channel = new Channel<TerminalEvent>();
  channel.onmessage = onEvent;
  await invokeSafeVoid('terminal_run', { command, onEvent: channel }, { silent: true });
}

/** Kills the running command. Resolves to whether there was one. */
export async function terminalCancel(): Promise<boolean> {
  return invokeSafe<boolean>('terminal_cancel', undefined, { silent: true });
}

/** The directory the next command starts in. */
export async function terminalCwd(): Promise<string> {
  return invokeSafe<string>('terminal_cwd', undefined, { silent: true });
}
