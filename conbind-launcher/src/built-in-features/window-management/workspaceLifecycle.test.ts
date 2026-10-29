/** @vitest-environment jsdom */
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockIndexItem = vi.hoisted(() => vi.fn());
const mockDeleteItem = vi.hoisted(() => vi.fn());
const mockRegisterCommand = vi.hoisted(() => vi.fn());
const mockUnregisterCommand = vi.hoisted(() => vi.fn());
const mockShortcutUnregister = vi.hoisted(() => vi.fn());
const mockCapture = vi.hoisted(() => vi.fn());
const mockOpen = vi.hoisted(() => vi.fn());
const mockShowHUD = vi.hoisted(() => vi.fn());
const mockReport = vi.hoisted(() => vi.fn());

vi.mock('../../services/search/SearchService', () => ({
  searchService: { indexItem: mockIndexItem, deleteItem: mockDeleteItem },
}));
vi.mock('../../services/extension/commandService.svelte', () => ({
  commandService: {
    registerCommand: mockRegisterCommand,
    unregisterCommand: mockUnregisterCommand,
  },
}));
vi.mock('../shortcuts/shortcutService', () => ({
  shortcutService: { unregister: mockShortcutUnregister },
}));
vi.mock('../../services/windowManagement/windowManagementService', () => ({
  windowManagementService: { captureWorkspace: mockCapture, openWorkspace: mockOpen },
}));
vi.mock('../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: { showHUD: mockShowHUD, report: mockReport },
}));
vi.mock('../../services/log/logService', () => ({
  logService: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

import {
  openWorkspace,
  saveCurrentWorkspace,
  updateWorkspace,
  syncWorkspaceToIndex,
  removeWorkspaceFromIndex,
  deleteWorkspace,
  renameWorkspace,
} from './workspaceLifecycle';
import { windowManagementState } from './state.svelte';
import type { IStorageService } from 'conbind-sdk/contracts';

function makeStore(): IStorageService {
  return {
    get: vi.fn(async () => null),
    set: vi.fn(async () => {}),
    delete: vi.fn(async () => true),
    getAll: vi.fn(async () => ({})),
    clear: vi.fn(async () => 0),
  } as unknown as IStorageService;
}

const captured = {
  windows: [
    {
      appId: 'app.a',
      appName: 'A',
      titleHint: 'One',
      monitor: 0,
      rect: { x: 0, y: 0, width: 0.5, height: 1 },
    },
    {
      appId: 'app.b',
      appName: 'B',
      titleHint: 'Two',
      monitor: 0,
      rect: { x: 0.5, y: 0, width: 0.5, height: 1 },
    },
  ],
  link: [0, 1] as [number, number],
};

describe('workspaceLifecycle', () => {
  let store: IStorageService;

  beforeEach(() => {
    vi.clearAllMocks();
    store = makeStore();
    windowManagementState.workspaces = [];
    windowManagementState.activeWorkspaceId = null;
    mockCapture.mockResolvedValue(captured);
    mockOpen.mockResolvedValue({ placed: 2, launched: 0, missing: [], ids: ['a', 'b'] });
  });

  describe('saveCurrentWorkspace', () => {
    it('captures, stores under the next free name, indexes it and shows a HUD', async () => {
      const ws = await saveCurrentWorkspace(store);
      expect(ws?.name).toBe('Workspace 1');
      expect(ws?.link).toEqual([0, 1]);
      expect(windowManagementState.workspaces).toHaveLength(1);
      expect(mockIndexItem).toHaveBeenCalledWith(
        expect.objectContaining({
          id: `cmd_window-management_workspace_${ws!.id}`,
          name: 'Workspace 1',
          extension: 'window-management',
        }),
      );
      expect(mockShowHUD).toHaveBeenCalled();
    });

    it('reports a capture failure and saves nothing', async () => {
      mockCapture.mockRejectedValue(new Error('no windows'));
      expect(await saveCurrentWorkspace(store)).toBeNull();
      expect(windowManagementState.workspaces).toHaveLength(0);
      expect(mockReport).toHaveBeenCalledWith(expect.objectContaining({ severity: 'error' }));
    });
  });

  describe('openWorkspace', () => {
    it('opens the windows with their link and marks the workspace active', async () => {
      const ws = (await saveCurrentWorkspace(store))!;
      vi.clearAllMocks();
      await openWorkspace(ws, store);
      expect(mockOpen).toHaveBeenCalledWith(ws.windows, [0, 1]);
      expect(windowManagementState.activeWorkspaceId).toBe(ws.id);
      expect(mockShowHUD).toHaveBeenCalledTimes(1);
    });

    it('tells the user which apps never showed up', async () => {
      const ws = (await saveCurrentWorkspace(store))!;
      mockOpen.mockResolvedValue({ placed: 1, launched: 1, missing: ['B'], ids: ['a', null] });
      mockShowHUD.mockClear();
      await openWorkspace(ws, store);
      expect(mockShowHUD).toHaveBeenCalledTimes(1);
      expect(String(mockShowHUD.mock.calls[0][0])).not.toBe('');
      expect(mockReport).not.toHaveBeenCalled();
    });

    it('reports an open failure', async () => {
      const ws = (await saveCurrentWorkspace(store))!;
      mockOpen.mockRejectedValue(new Error('denied'));
      await openWorkspace(ws, store);
      expect(mockReport).toHaveBeenCalledWith(expect.objectContaining({ severity: 'error' }));
    });
  });

  describe('updateWorkspace', () => {
    it('replaces the windows of an existing workspace', async () => {
      const ws = (await saveCurrentWorkspace(store))!;
      mockCapture.mockResolvedValue({ windows: [captured.windows[0]], link: null });
      await updateWorkspace(ws, store);
      const updated = windowManagementState.workspaces[0];
      expect(updated.windows).toHaveLength(1);
      expect(updated.link).toBeNull();
      expect(updated.name).toBe(ws.name);
    });
  });

  describe('index registration', () => {
    it('registers a command that opens the current copy of the workspace', async () => {
      const ws = (await saveCurrentWorkspace(store))!;
      mockRegisterCommand.mockClear();
      await syncWorkspaceToIndex(ws, store);
      const [objectId, handler, extensionId] = mockRegisterCommand.mock.calls[0];
      expect(objectId).toBe(`cmd_window-management_workspace_${ws.id}`);
      expect(extensionId).toBe('window-management');

      mockCapture.mockResolvedValue({ windows: [captured.windows[0]], link: null });
      await updateWorkspace(ws, store);
      mockOpen.mockClear();
      expect(await handler.execute()).toEqual({ type: 'no-view' });
      expect(mockOpen).toHaveBeenCalledWith([captured.windows[0]], null);
    });

    it('removeWorkspaceFromIndex unregisters search, command and shortcut', async () => {
      await removeWorkspaceFromIndex('abc');
      expect(mockDeleteItem).toHaveBeenCalledWith('cmd_window-management_workspace_abc');
      expect(mockUnregisterCommand).toHaveBeenCalledWith('cmd_window-management_workspace_abc');
      expect(mockShortcutUnregister).toHaveBeenCalledWith('cmd_window-management_workspace_abc');
    });
  });

  describe('deleteWorkspace / renameWorkspace', () => {
    it('deleteWorkspace removes it from state and the index', async () => {
      const ws = (await saveCurrentWorkspace(store))!;
      await deleteWorkspace(ws.id, store);
      expect(windowManagementState.workspaces).toHaveLength(0);
      expect(mockDeleteItem).toHaveBeenCalledWith(`cmd_window-management_workspace_${ws.id}`);
    });

    it('renameWorkspace renames and re-indexes under the new name', async () => {
      const ws = (await saveCurrentWorkspace(store))!;
      mockIndexItem.mockClear();
      await renameWorkspace(ws.id, 'Focus', store);
      expect(windowManagementState.workspaces[0].name).toBe('Focus');
      expect(mockIndexItem).toHaveBeenCalledWith(expect.objectContaining({ name: 'Focus' }));
    });

    it('renameWorkspace throws for a name that is already taken', async () => {
      const first = (await saveCurrentWorkspace(store))!;
      await saveCurrentWorkspace(store);
      await expect(renameWorkspace(first.id, 'workspace 2', store)).rejects.toThrow('already');
    });
  });
});
