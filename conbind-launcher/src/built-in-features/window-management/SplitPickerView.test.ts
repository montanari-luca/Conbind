/** @vitest-environment jsdom */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { tick } from 'svelte';

const mockListWindows = vi.fn();
const mockSplit = vi.fn();
const mockHideWindow = vi.fn();
const mockResetLauncherState = vi.fn();
const mockShowHUD = vi.fn();
const mockReport = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
  transformCallback: vi.fn(() => 0),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../../services/windowManagement/windowManagementService', () => ({
  windowManagementService: {
    listWindows: () => mockListWindows(),
    split: (id: string, axis: string) => mockSplit(id, axis),
  },
}));

vi.mock('../../lib/ipc/commands', () => ({
  hideWindow: () => mockHideWindow(),
}));

vi.mock('../../lib/launcher/launcherReset', () => ({
  resetLauncherState: () => mockResetLauncherState(),
}));

vi.mock('../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: {
    showHUD: (msg: string) => mockShowHUD(msg),
    report: (r: any) => mockReport(r),
  },
}));

vi.mock('../../services/i18n', () => ({
  t: (key: string) => key,
}));

import SplitPickerView from './SplitPickerView.svelte';
import { searchStores } from '../../services/search/stores/search.svelte';
import { windowManagementState } from './state.svelte';

const win = (over: Record<string, unknown>) => ({
  id: 'win-x',
  pid: 1,
  appName: 'App',
  appBundleId: null,
  title: 'Title',
  isMinimized: false,
  isFocused: false,
  appIcon: null,
  ...over,
});

const SAMPLE_WINDOWS = [
  win({ id: 'front', appName: 'Safari', title: 'Frontmost page', isFocused: true }),
  win({ id: 'mail', appName: 'Mail', title: 'Inbox' }),
  win({ id: 'hidden', appName: 'Notes', title: 'Minimized note', isMinimized: true }),
  win({ id: 'code', appName: 'Code', title: 'index.ts' }),
];

async function flush(): Promise<void> {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 10));
  await tick();
}

describe('SplitPickerView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    searchStores.query = '';
    windowManagementState.splitAxis = 'side';
    mockListWindows.mockResolvedValue([...SAMPLE_WINDOWS]);
    mockSplit.mockResolvedValue(undefined);
    mockHideWindow.mockResolvedValue(undefined);
  });

  it('offers every window except the frontmost and minimized ones', async () => {
    const { queryByText } = render(SplitPickerView);
    await flush();

    expect(queryByText('Inbox')).toBeTruthy();
    expect(queryByText('index.ts')).toBeTruthy();
    expect(queryByText('Frontmost page')).toBeNull();
    expect(queryByText('Minimized note')).toBeNull();
  });

  it('shows an empty state when there is nothing to split with', async () => {
    mockListWindows.mockResolvedValue([SAMPLE_WINDOWS[0]]);
    const { getByText } = render(SplitPickerView);
    await flush();

    expect(getByText('features.window_management.no_split_windows')).toBeTruthy();
  });

  it('splits side by side with the chosen window and closes the launcher', async () => {
    const { getByText } = render(SplitPickerView);
    await flush();

    await fireEvent.click(getByText('Inbox'));
    await flush();

    expect(mockSplit).toHaveBeenCalledWith('mail', 'side');
    expect(mockShowHUD).toHaveBeenCalledWith('features.window_management.hud_split');
    expect(mockHideWindow).toHaveBeenCalled();
    expect(mockResetLauncherState).toHaveBeenCalled();
  });

  it('uses the axis chosen by the command that opened it', async () => {
    windowManagementState.splitAxis = 'stack';
    const { getByText } = render(SplitPickerView);
    await flush();

    await fireEvent.click(getByText('index.ts'));
    await flush();

    expect(mockSplit).toHaveBeenCalledWith('code', 'stack');
  });

  it('splits with the selected row on Enter', async () => {
    render(SplitPickerView);
    await flush();

    await fireEvent.keyDown(window, { key: 'ArrowDown' });
    await fireEvent.keyDown(window, { key: 'Enter' });
    await flush();

    expect(mockSplit).toHaveBeenCalledWith('code', 'side');
  });

  it('filters by the search query', async () => {
    const { queryByText } = render(SplitPickerView);
    await flush();

    searchStores.query = 'mail';
    await flush();

    expect(queryByText('Inbox')).toBeTruthy();
    expect(queryByText('index.ts')).toBeNull();
  });

  it('reports a failed split and keeps the launcher open', async () => {
    mockSplit.mockRejectedValue(new Error('window is gone'));
    const { getByText } = render(SplitPickerView);
    await flush();

    await fireEvent.click(getByText('Inbox'));
    await flush();

    expect(mockReport).toHaveBeenCalledWith(
      expect.objectContaining({
        severity: 'error',
        context: expect.objectContaining({
          message: 'features.window_management.ui.split_failed',
        }),
      }),
    );
    expect(mockHideWindow).not.toHaveBeenCalled();
  });
});
