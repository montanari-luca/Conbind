// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: vi.fn().mockResolvedValue('0.1.0') }));
const { openUrlMock } = vi.hoisted(() => ({ openUrlMock: vi.fn() }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: openUrlMock }));

const { runUpdateCheckMock } = vi.hoisted(() => ({
  runUpdateCheckMock: vi.fn(),
}));

vi.mock('../../../services/update/updateService', () => ({
  runUpdateCheck: runUpdateCheckMock,
}));

import AboutTab from './AboutTab.svelte';
import type { SettingsHandler } from '../settingsHandlers.svelte';
import {
  appUpdateState,
  destroyAppUpdateStore,
} from '../../../services/update/appUpdateStore.svelte';

function createMockHandler(): SettingsHandler {
  return {
    settings: {
      updates: { autoCheck: true },
    },
    updateAutoCheck: vi.fn(),
    activeTab: 'about',
  } as unknown as SettingsHandler;
}

describe('AboutTab update download button', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    destroyAppUpdateStore();
    invokeMock.mockResolvedValue(null);
  });

  it('shows a Download button after Check for Updates finds a newer version', async () => {
    runUpdateCheckMock.mockResolvedValueOnce({
      kind: 'available',
      version: '0.2.0',
      downloadUrl: 'https://example.test/conbind-0.2.0.dmg',
    });

    const handler = createMockHandler();
    render(AboutTab, { props: { handler } });

    expect(screen.queryByRole('button', { name: /^download$/i })).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: /check for updates/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^download$/i })).toBeTruthy();
    });

    // Clicking Download opens the direct link from the release feed
    await fireEvent.click(screen.getByRole('button', { name: /^download$/i }));
    expect(openUrlMock).toHaveBeenCalledWith('https://example.test/conbind-0.2.0.dmg');
  });

  it('shows the Download button on mount if an update was already found', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'app_updater_get_pending') {
        return { version: '0.2.0', downloadUrl: 'https://example.test/x' };
      }
      return null;
    });

    const handler = createMockHandler();
    render(AboutTab, { props: { handler } });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^download$/i })).toBeTruthy();
    });
  });

  it('does not show the Download button when up to date', async () => {
    runUpdateCheckMock.mockResolvedValueOnce({ kind: 'up-to-date' });

    const handler = createMockHandler();
    render(AboutTab, { props: { handler } });

    const checkBtn = screen.getByRole('button', { name: /check for updates/i });
    await fireEvent.click(checkBtn);

    await waitFor(() => {
      expect(screen.getByText("You're running the latest version.")).toBeTruthy();
    });

    expect(screen.queryByRole('button', { name: /^download$/i })).toBeNull();
  });

  it('does not show the Download button when update check fails', async () => {
    runUpdateCheckMock.mockResolvedValueOnce({ kind: 'error', message: 'network unreachable' });

    const handler = createMockHandler();
    render(AboutTab, { props: { handler } });

    const checkBtn = screen.getByRole('button', { name: /check for updates/i });
    await fireEvent.click(checkBtn);

    await waitFor(() => {
      expect(screen.getByText(/Update check failed: network unreachable/i)).toBeTruthy();
    });

    expect(screen.queryByRole('button', { name: /^download$/i })).toBeNull();
  });
});
