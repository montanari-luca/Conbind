import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('../../lib/ipc/commands', () => ({
  batteryEstimate: vi.fn(),
}));

import { batteryEstimate } from '../../lib/ipc/commands';
import { batteryState } from './state.svelte';

const SNAPSHOT = {
  present: true,
  percent: 64,
  state: 'discharging' as const,
  secondsToEmpty: 7200,
  secondsToFull: null,
};

describe('batteryState', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    vi.mocked(batteryEstimate).mockResolvedValue(SNAPSHOT);
  });

  afterEach(() => {
    batteryState.reset();
    vi.useRealTimers();
  });

  it('polls every five seconds until stopped', async () => {
    batteryState.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(batteryState.snapshot?.secondsToEmpty).toBe(7200);
    expect(batteryEstimate).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(10_000);
    expect(batteryEstimate).toHaveBeenCalledTimes(3);

    batteryState.stop();
    await vi.advanceTimersByTimeAsync(20_000);
    expect(batteryEstimate).toHaveBeenCalledTimes(3);
  });

  it('does not start a second loop when already running', async () => {
    batteryState.start();
    batteryState.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(batteryEstimate).toHaveBeenCalledTimes(1);
  });

  it('flags a failed read and recovers on the next one', async () => {
    vi.mocked(batteryEstimate).mockResolvedValueOnce(null);
    batteryState.start();
    await vi.advanceTimersByTimeAsync(0);
    expect(batteryState.failed).toBe(true);

    await vi.advanceTimersByTimeAsync(5000);
    expect(batteryState.failed).toBe(false);
    expect(batteryState.snapshot).not.toBeNull();
  });

  it('reset clears the snapshot', async () => {
    batteryState.start();
    await vi.advanceTimersByTimeAsync(0);
    batteryState.reset();
    expect(batteryState.snapshot).toBeNull();
  });
});
