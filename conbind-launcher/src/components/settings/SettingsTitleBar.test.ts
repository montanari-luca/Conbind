// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { platformMock, windowMock } = vi.hoisted(() => ({
  platformMock: vi.fn(),
  windowMock: {
    minimize: vi.fn().mockResolvedValue(undefined),
    toggleMaximize: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    isMaximized: vi.fn().mockResolvedValue(false),
    onResized: vi.fn().mockResolvedValue(() => {}),
  },
}));

vi.mock('@tauri-apps/plugin-os', () => ({ platform: platformMock }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => windowMock }));
vi.mock('../../services/log/logService', () => ({
  logService: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

import SettingsTitleBar from './SettingsTitleBar.svelte';

describe('SettingsTitleBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    windowMock.isMaximized.mockResolvedValue(false);
  });

  it('is a drag region showing the window title', () => {
    platformMock.mockResolvedValue('macos');
    const { container } = render(SettingsTitleBar, { props: { title: 'Conbind Settings' } });
    expect(screen.getByText('Conbind Settings')).toBeTruthy();
    expect(container.querySelector('.titlebar')?.hasAttribute('data-tauri-drag-region')).toBe(true);
  });

  it('leaves window controls to the native traffic lights on macOS', async () => {
    platformMock.mockResolvedValue('macos');
    render(SettingsTitleBar, { props: { title: 'Conbind Settings' } });
    await waitFor(() => expect(platformMock).toHaveBeenCalled());
    expect(screen.queryByRole('button')).toBeNull();
  });

  it.each(['windows', 'linux'])('draws minimize, maximize and close on %s', async (os) => {
    platformMock.mockResolvedValue(os);
    render(SettingsTitleBar, { props: { title: 'Conbind Settings' } });

    await fireEvent.click(await screen.findByRole('button', { name: 'Minimize' }));
    expect(windowMock.minimize).toHaveBeenCalledTimes(1);

    await fireEvent.click(screen.getByRole('button', { name: 'Maximize' }));
    expect(windowMock.toggleMaximize).toHaveBeenCalledTimes(1);

    await fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(windowMock.close).toHaveBeenCalledTimes(1);
  });

  it('offers Restore while the window is maximized', async () => {
    platformMock.mockResolvedValue('windows');
    windowMock.isMaximized.mockResolvedValue(true);
    render(SettingsTitleBar, { props: { title: 'Conbind Settings' } });
    expect(await screen.findByRole('button', { name: 'Restore window' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Maximize' })).toBeNull();
  });
});
