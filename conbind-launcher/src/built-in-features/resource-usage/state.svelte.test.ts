import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('../../lib/ipc/commands', () => ({
  resourceUsageSnapshot: vi.fn(),
}));

import { resourceUsageSnapshot } from '../../lib/ipc/commands';
import { resourceUsageState } from './state.svelte';

const SNAPSHOT = {
  cpu: { percent: 10, cores: 8 },
  memory: { used: 1, total: 2, swapUsed: 0, swapTotal: 0 },
  network: { rxPerSec: 0, txPerSec: 0, totalRx: 0, totalTx: 0 },
  disks: [],
  gpus: [],
};

describe('resourceUsageState', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    vi.mocked(resourceUsageSnapshot).mockResolvedValue(SNAPSHOT);
  });

  afterEach(() => {
    resourceUsageState.reset();
    vi.useRealTimers();
  });

  it('polls once a second until stopped', async () => {
    resourceUsageState.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(resourceUsageState.snapshot?.cpu.cores).toBe(8);
    expect(resourceUsageSnapshot).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(2000);
    expect(resourceUsageSnapshot).toHaveBeenCalledTimes(3);

    resourceUsageState.stop();
    await vi.advanceTimersByTimeAsync(5000);
    expect(resourceUsageSnapshot).toHaveBeenCalledTimes(3);
  });

  it('does not start a second loop when already running', async () => {
    resourceUsageState.start();
    resourceUsageState.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(resourceUsageSnapshot).toHaveBeenCalledTimes(1);
  });

  it('flags a failed poll but keeps the last good snapshot', async () => {
    resourceUsageState.start();
    await vi.advanceTimersByTimeAsync(0);
    vi.mocked(resourceUsageSnapshot).mockResolvedValue(null);
    await vi.advanceTimersByTimeAsync(1000);
    expect(resourceUsageState.failed).toBe(true);
    expect(resourceUsageState.snapshot).not.toBeNull();
  });
});
