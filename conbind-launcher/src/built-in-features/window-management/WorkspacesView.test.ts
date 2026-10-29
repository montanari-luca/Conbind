/** @vitest-environment jsdom */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { tick } from 'svelte';

const mockOpenWorkspace = vi.fn();
const mockSaveCurrent = vi.fn();
const mockUpdateWorkspace = vi.fn();
const mockDeleteWorkspace = vi.fn();
const mockRenameWorkspace = vi.fn();
const mockHideWindow = vi.fn();
const mockResetLauncherState = vi.fn();
const mockRegisterAction = vi.fn();
const mockUnregisterAction = vi.fn();
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
vi.mock('./workspaceLifecycle', () => ({
  openWorkspace: (...a: unknown[]) => mockOpenWorkspace(...a),
  saveCurrentWorkspace: (...a: unknown[]) => mockSaveCurrent(...a),
  updateWorkspace: (...a: unknown[]) => mockUpdateWorkspace(...a),
  deleteWorkspace: (...a: unknown[]) => mockDeleteWorkspace(...a),
  renameWorkspace: (...a: unknown[]) => mockRenameWorkspace(...a),
  describeWorkspace: (w: { windows: unknown[] }) => `${w.windows.length} windows`,
}));
vi.mock('../../lib/ipc/commands', () => ({ hideWindow: () => mockHideWindow() }));
vi.mock('../../lib/launcher/launcherReset', () => ({
  resetLauncherState: () => mockResetLauncherState(),
}));
vi.mock('../../services/action/actionService.svelte', () => ({
  actionService: {
    registerAction: (a: any) => mockRegisterAction(a),
    unregisterAction: (id: string) => mockUnregisterAction(id),
  },
}));
vi.mock('../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: {
    showHUD: (m: string) => mockShowHUD(m),
    report: (r: any) => mockReport(r),
  },
}));
vi.mock('../../services/i18n', () => ({ t: (key: string) => key }));

import WorkspacesView from './WorkspacesView.svelte';
import { windowManagementState } from './state.svelte';

const workspace = (id: string, name: string) => ({
  id,
  name,
  windows: [
    {
      appId: 'a',
      appName: 'A',
      titleHint: '',
      monitor: 0,
      rect: { x: 0, y: 0, width: 1, height: 1 },
    },
  ],
  link: null,
  created: 1,
  updated: 1,
});

async function flush(): Promise<void> {
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 10));
  await tick();
}

describe('WorkspacesView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    windowManagementState.workspaces = [workspace('1', 'Dev'), workspace('2', 'Mail')];
    windowManagementState.setWorkspaceIndex(0);
    mockOpenWorkspace.mockResolvedValue(undefined);
    mockHideWindow.mockResolvedValue(undefined);
  });

  it('lists the saved workspaces', async () => {
    const { getByText } = render(WorkspacesView);
    await flush();
    expect(getByText('Dev')).toBeTruthy();
    expect(getByText('Mail')).toBeTruthy();
  });

  it('shows an empty state without workspaces', async () => {
    windowManagementState.workspaces = [];
    const { getByText } = render(WorkspacesView);
    await flush();
    expect(getByText('features.window_management.no_workspaces')).toBeTruthy();
  });

  it('opens the selected workspace on Enter after hiding the launcher', async () => {
    render(WorkspacesView);
    await flush();
    await fireEvent.keyDown(window, { key: 'ArrowDown' });
    await fireEvent.keyDown(window, { key: 'Enter' });
    await flush();

    expect(mockHideWindow).toHaveBeenCalled();
    expect(mockResetLauncherState).toHaveBeenCalled();
    expect(mockOpenWorkspace).toHaveBeenCalledWith(
      expect.objectContaining({ id: '2', name: 'Mail' }),
      undefined,
    );
  });

  it('registers actions for the selected workspace', async () => {
    render(WorkspacesView);
    await flush();
    const ids = mockRegisterAction.mock.calls.map(([a]) => a.id);
    expect(ids).toEqual(
      expect.arrayContaining([
        'window-management:save-workspace',
        'window-management:open-workspace',
        'window-management:update-workspace',
        'window-management:rename-workspace',
        'window-management:delete-workspace',
      ]),
    );
  });

  it('delete action removes the workspace and confirms with a HUD', async () => {
    mockDeleteWorkspace.mockResolvedValue(undefined);
    render(WorkspacesView);
    await flush();
    const del = mockRegisterAction.mock.calls
      .map(([a]) => a)
      .find((a) => a.id === 'window-management:delete-workspace');
    await del.execute();
    expect(mockDeleteWorkspace).toHaveBeenCalledWith('1', undefined);
    expect(mockShowHUD).toHaveBeenCalledWith('features.window_management.hud_workspace_deleted');
  });

  it('save action saves the current windows', async () => {
    mockSaveCurrent.mockResolvedValue(null);
    render(WorkspacesView);
    await flush();
    const save = mockRegisterAction.mock.calls
      .map(([a]) => a)
      .find((a) => a.id === 'window-management:save-workspace');
    await save.execute();
    expect(mockSaveCurrent).toHaveBeenCalled();
  });

  it('update action overwrites the selected workspace', async () => {
    mockUpdateWorkspace.mockResolvedValue(undefined);
    render(WorkspacesView);
    await flush();
    const update = mockRegisterAction.mock.calls
      .map(([a]) => a)
      .find((a) => a.id === 'window-management:update-workspace');
    await update.execute();
    expect(mockUpdateWorkspace).toHaveBeenCalledWith(
      expect.objectContaining({ id: '1' }),
      undefined,
    );
  });
});
