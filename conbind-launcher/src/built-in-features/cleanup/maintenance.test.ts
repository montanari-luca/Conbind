import { beforeEach, describe, expect, it, vi } from 'vitest';

const ipc = vi.hoisted(() => ({
  cleanupEmptyTrash: vi.fn(async () => undefined),
  cleanupNetwork: vi.fn(async () => 'done' as 'done' | 'cancelled'),
  setFocusLock: vi.fn(async () => undefined),
}));
vi.mock('../../lib/ipc/commands', () => ipc);

const feedback = vi.hoisted(() => ({
  confirmAlert: vi.fn(async () => true),
  showHUD: vi.fn(async () => undefined),
}));
vi.mock('../../services/feedback/feedbackService.svelte', () => ({ feedbackService: feedback }));
vi.mock('../../services/i18n', () => ({ t: (key: string) => key }));

import { runEmptyTrash, runNetworkCleanup } from './maintenance';

beforeEach(() => {
  vi.clearAllMocks();
  feedback.confirmAlert.mockResolvedValue(true);
  ipc.cleanupNetwork.mockResolvedValue('done');
});

describe('runNetworkCleanup', () => {
  it('does nothing when the user declines', async () => {
    feedback.confirmAlert.mockResolvedValue(false);
    await runNetworkCleanup();
    expect(ipc.cleanupNetwork).not.toHaveBeenCalled();
    expect(feedback.showHUD).not.toHaveBeenCalled();
  });

  it('confirms with the danger style, runs, and announces success', async () => {
    await runNetworkCleanup();
    expect(feedback.confirmAlert).toHaveBeenCalledWith(
      expect.objectContaining({ variant: 'danger' }),
    );
    expect(ipc.cleanupNetwork).toHaveBeenCalledOnce();
    expect(feedback.showHUD).toHaveBeenCalledWith('features.cleanup.network.done');
  });

  it('stays quiet when the OS password prompt was dismissed', async () => {
    ipc.cleanupNetwork.mockResolvedValue('cancelled');
    await runNetworkCleanup();
    expect(feedback.showHUD).not.toHaveBeenCalled();
  });

  it('always releases the focus lock, even if the dialog throws', async () => {
    feedback.confirmAlert.mockRejectedValue(new Error('boom'));
    await expect(runNetworkCleanup()).rejects.toThrow('boom');
    expect(ipc.setFocusLock).toHaveBeenLastCalledWith(false);
  });
});

describe('runEmptyTrash', () => {
  it('does nothing when the user declines', async () => {
    feedback.confirmAlert.mockResolvedValue(false);
    await runEmptyTrash();
    expect(ipc.cleanupEmptyTrash).not.toHaveBeenCalled();
  });

  it('empties the trash and announces it once confirmed', async () => {
    await runEmptyTrash();
    expect(ipc.cleanupEmptyTrash).toHaveBeenCalledOnce();
    expect(feedback.showHUD).toHaveBeenCalledWith('features.cleanup.empty_trash.done');
  });
});
