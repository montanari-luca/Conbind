import { homeDir } from '@tauri-apps/api/path';
import { terminalCancel, terminalCwd, terminalRun } from '../../lib/ipc/commands';
import type { TerminalEvent } from '../../lib/ipc/commands';
import { extractErrorMessage } from '../../lib/errors';

// Output is capped so a runaway command cannot grow the webview without bound.
const MAX_BLOCKS = 100;
const MAX_LINES_PER_BLOCK = 5000;
const FLUSH_MS = 50;

export interface OutputLine {
  stream: 'stdout' | 'stderr';
  text: string;
}

export interface TerminalBlock {
  id: number;
  command: string;
  cwd: string;
  lines: OutputLine[];
  truncated: boolean;
  status: 'running' | 'done' | 'cancelled';
  exitCode: number | null;
}

class TerminalStateStore {
  blocks = $state<TerminalBlock[]>([]);
  cwd = $state('');
  home = $state<string | null>(null);

  /** Typed commands, oldest first, for the arrow-key history. */
  history: string[] = [];
  /** Set by the search row, picked up by the view when it opens. */
  pending: string | null = null;

  private nextId = 1;
  private buffer: OutputLine[] = [];
  private flushTimer: ReturnType<typeof setTimeout> | null = null;

  get running(): boolean {
    return this.blocks.some((b) => b.status === 'running');
  }

  queue(command: string): void {
    this.pending = command;
  }

  takePending(): string | null {
    const command = this.pending;
    this.pending = null;
    return command;
  }

  /** Loads the starting directory and home once, so the prompt is right before any command. */
  async init(): Promise<void> {
    if (!this.home) this.home = await homeDir().catch(() => null);
    if (!this.cwd) this.cwd = await terminalCwd().catch(() => '');
  }

  async run(raw: string): Promise<void> {
    const command = raw.trim();
    if (!command || this.running) return;
    if (this.history[this.history.length - 1] !== command) this.history.push(command);

    if (command === 'clear' || command === 'cls') {
      this.blocks = [];
      return;
    }

    this.blocks.push({
      id: this.nextId++,
      command,
      cwd: this.cwd,
      lines: [],
      truncated: false,
      status: 'running',
      exitCode: null,
    });
    if (this.blocks.length > MAX_BLOCKS) this.blocks.splice(0, this.blocks.length - MAX_BLOCKS);
    const block = this.blocks[this.blocks.length - 1];

    try {
      await terminalRun(command, (event) => this.onEvent(block, event));
    } catch (err) {
      this.flush(block);
      block.lines.push({ stream: 'stderr', text: extractErrorMessage(err) });
      block.status = 'done';
      block.exitCode = null;
    }
  }

  async cancel(): Promise<void> {
    if (!this.running) return;
    await terminalCancel().catch(() => false);
  }

  private onEvent(block: TerminalBlock, event: TerminalEvent): void {
    if (event.type === 'chunk') {
      this.buffer.push({ stream: event.stream, text: event.data });
      this.flushTimer ??= setTimeout(() => this.flush(block), FLUSH_MS);
      return;
    }
    this.flush(block);
    this.cwd = event.cwd;
    block.exitCode = event.exitCode;
    block.status = event.cancelled ? 'cancelled' : 'done';
  }

  private flush(block: TerminalBlock): void {
    if (this.flushTimer) clearTimeout(this.flushTimer);
    this.flushTimer = null;
    if (this.buffer.length === 0) return;
    const room = MAX_LINES_PER_BLOCK - block.lines.length;
    if (this.buffer.length > room) block.truncated = true;
    if (room > 0) block.lines.push(...this.buffer.slice(0, room));
    this.buffer = [];
  }
}

export const terminalState = new TerminalStateStore();
