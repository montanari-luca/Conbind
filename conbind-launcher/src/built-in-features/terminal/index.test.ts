import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../../services/log/logService', () => ({
  logService: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));
vi.mock('./TerminalView.svelte', () => ({ default: {} }));
vi.mock('@tauri-apps/api/path', () => ({ homeDir: vi.fn() }));
vi.mock('../../lib/ipc/commands', () => ({
  terminalRun: vi.fn(),
  terminalCancel: vi.fn(),
  terminalCwd: vi.fn(),
}));

import extension from './index';
import { terminalState } from './terminalState.svelte';
import {
  executeBuiltinSearchResult,
  searchBuiltinProviders,
} from '../../services/search/builtinSearchProviders';

const ROW_ID = 'terminal_run';

describe('terminal built-in', () => {
  beforeEach(() => {
    terminalState.pending = null;
  });

  it('returns one top row for a > query', async () => {
    const hits = await searchBuiltinProviders('> cd /tmp', () => true);
    const mine = hits.filter((h) => h.extensionId === 'terminal');
    expect(mine).toHaveLength(1);
    expect(mine[0]).toMatchObject({
      id: ROW_ID,
      icon: 'icon:terminal',
      priority: 'top',
      actionPayload: { command: 'cd /tmp' },
    });
  });

  it('returns nothing without the prefix or a command', async () => {
    for (const q of ['ls', '>', '>  ']) {
      const hits = await searchBuiltinProviders(q, () => true);
      expect(hits.filter((h) => h.extensionId === 'terminal')).toEqual([]);
    }
  });

  it('queues the command and opens the view on Enter', async () => {
    const navigateToView = vi.fn();
    await extension.initialize({
      getService: () => ({ navigateToView }),
    } as unknown as Parameters<typeof extension.initialize>[0]);
    await executeBuiltinSearchResult('terminal', ROW_ID, { command: 'pwd' });
    expect(terminalState.takePending()).toBe('pwd');
    expect(navigateToView).toHaveBeenCalledWith('terminal/TerminalView');
  });
});
