/** @vitest-environment jsdom */
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock logService before importing the module under test
vi.mock('../../services/log/logService', () => ({
  logService: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

import { WindowManagementState, cleanWorkspaceName, MAX_WORKSPACE_NAME } from './state.svelte';
import type { IStorageService } from 'conbind-sdk/contracts';
import type { CapturedWorkspace, WindowBounds } from '../../lib/ipc/commands';

function makeStoreMock(data: Record<string, string> = {}): IStorageService {
  const store = { ...data };
  return {
    get: vi.fn(async (key: string) => store[key] ?? null),
    set: vi.fn(async (key: string, value: string) => {
      store[key] = value;
    }),
    delete: vi.fn(async () => true),
    getAll: vi.fn(async () => store),
    clear: vi.fn(async () => 0),
  } as unknown as IStorageService;
}

describe('WindowManagementState', () => {
  let state: WindowManagementState;
  let storeMock: IStorageService;

  beforeEach(() => {
    vi.clearAllMocks();
    state = new WindowManagementState();
    storeMock = makeStoreMock();
  });

  describe('loadFromStorage', () => {
    it('initializes with empty arrays when storage is empty', async () => {
      await state.loadFromStorage(storeMock);
      expect(state.customLayouts).toEqual([]);
    });

    it('loads custom layouts from storage', async () => {
      const layouts = [
        { id: '1', name: 'My Layout', bounds: { x: 0, y: 0, width: 800, height: 600 } },
      ];
      storeMock = makeStoreMock({ custom_layouts: JSON.stringify(layouts) });
      await state.loadFromStorage(storeMock);
      expect(state.customLayouts).toEqual(layouts);
    });

    it('handles corrupted JSON gracefully — sets empty state', async () => {
      storeMock = makeStoreMock({ custom_layouts: 'not-json' });
      await state.loadFromStorage(storeMock);
      expect(state.customLayouts).toEqual([]);
    });
  });

  describe('addCustomLayout', () => {
    it('appends layout and persists to storage', async () => {
      const bounds: WindowBounds = { x: 0, y: 0, width: 1200, height: 800 };
      await state.loadFromStorage(storeMock);
      await state.addCustomLayout('Work Setup', bounds, storeMock);
      expect(state.customLayouts).toHaveLength(1);
      expect(state.customLayouts[0].name).toBe('Work Setup');
      expect(state.customLayouts[0].bounds).toEqual(bounds);
      expect(storeMock.set).toHaveBeenCalledWith(
        'custom_layouts',
        JSON.stringify(state.customLayouts),
      );
    });

    it('generates a unique id based on timestamp', async () => {
      const bounds: WindowBounds = { x: 0, y: 0, width: 800, height: 600 };
      await state.loadFromStorage(storeMock);
      await state.addCustomLayout('A', bounds, storeMock);
      await state.addCustomLayout('B', bounds, storeMock);
      const ids = state.customLayouts.map((l) => l.id);
      expect(new Set(ids).size).toBe(2);
    });
  });

  describe('deleteCustomLayout', () => {
    it('removes layout by id and persists to storage', async () => {
      const layouts = [
        { id: 'abc', name: 'Layout A', bounds: { x: 0, y: 0, width: 800, height: 600 } },
        { id: 'def', name: 'Layout B', bounds: { x: 0, y: 0, width: 1200, height: 800 } },
      ];
      storeMock = makeStoreMock({ custom_layouts: JSON.stringify(layouts) });
      await state.loadFromStorage(storeMock);
      await state.deleteCustomLayout('abc', storeMock);
      expect(state.customLayouts).toHaveLength(1);
      expect(state.customLayouts[0].id).toBe('def');
      expect(storeMock.set).toHaveBeenCalledWith(
        'custom_layouts',
        JSON.stringify(state.customLayouts),
      );
    });

    it('is a no-op for unknown id', async () => {
      await state.loadFromStorage(storeMock);
      await state.deleteCustomLayout('nonexistent', storeMock);
      expect(state.customLayouts).toEqual([]);
    });
  });

  describe('renameCustomLayout', () => {
    it('renames layout by id and persists to storage', async () => {
      const layouts = [
        { id: 'abc', name: 'Old Name', bounds: { x: 0, y: 0, width: 800, height: 600 } },
        { id: 'def', name: 'Layout B', bounds: { x: 0, y: 0, width: 1200, height: 800 } },
      ];
      storeMock = makeStoreMock({ custom_layouts: JSON.stringify(layouts) });
      await state.loadFromStorage(storeMock);
      await state.renameCustomLayout('abc', 'New Name', storeMock);
      expect(state.customLayouts).toHaveLength(2);
      expect(state.customLayouts[0].name).toBe('New Name');
      expect(state.customLayouts[1].name).toBe('Layout B');
      expect(storeMock.set).toHaveBeenCalledWith(
        'custom_layouts',
        JSON.stringify(state.customLayouts),
      );
    });

    it('is a no-op when name is empty or unchanged', async () => {
      const layouts = [
        { id: 'abc', name: 'Old Name', bounds: { x: 0, y: 0, width: 800, height: 600 } },
      ];
      storeMock = makeStoreMock({ custom_layouts: JSON.stringify(layouts) });
      await state.loadFromStorage(storeMock);
      await state.renameCustomLayout('abc', '   ', storeMock);
      expect(state.customLayouts[0].name).toBe('Old Name');
      expect(storeMock.set).not.toHaveBeenCalled();
    });

    it('is a no-op for unknown id', async () => {
      await state.loadFromStorage(storeMock);
      await state.renameCustomLayout('nonexistent', 'Name', storeMock);
      expect(state.customLayouts).toEqual([]);
    });
  });

  describe('workspaces', () => {
    const captured = (count = 2): CapturedWorkspace => ({
      windows: Array.from({ length: count }, (_, i) => ({
        appId: `app.${i}`,
        appName: `App ${i}`,
        titleHint: `Title ${i}`,
        monitor: 0,
        rect: { x: i / count, y: 0, width: 1 / count, height: 1 },
      })),
      link: count === 2 ? ([0, 1] as [number, number]) : null,
    });

    it('cleanWorkspaceName trims and enforces 1 to 40 characters', () => {
      expect(cleanWorkspaceName('  Dev  ')).toBe('Dev');
      expect(cleanWorkspaceName('   ')).toBeNull();
      expect(cleanWorkspaceName('x'.repeat(MAX_WORKSPACE_NAME))).not.toBeNull();
      expect(cleanWorkspaceName('x'.repeat(MAX_WORKSPACE_NAME + 1))).toBeNull();
    });

    it('loads workspaces and the active id from storage', async () => {
      const ws = { id: 'a', name: 'Dev', windows: [], link: null, created: 1, updated: 1 };
      storeMock = makeStoreMock({
        workspaces: JSON.stringify([ws]),
        active_workspace: 'a',
      });
      await state.loadFromStorage(storeMock);
      expect(state.workspaces).toEqual([ws]);
      expect(state.activeWorkspace).toEqual(ws);
    });

    it('survives corrupted workspace JSON', async () => {
      storeMock = makeStoreMock({ workspaces: 'not-json' });
      await state.loadFromStorage(storeMock);
      expect(state.workspaces).toEqual([]);
    });

    it('addWorkspace persists, selects and activates the new workspace', async () => {
      await state.loadFromStorage(storeMock);
      const created = await state.addWorkspace('Dev', captured(), storeMock);
      expect(created.name).toBe('Dev');
      expect(created.link).toEqual([0, 1]);
      expect(state.workspaces).toHaveLength(1);
      expect(state.selectedWorkspace?.id).toBe(created.id);
      expect(state.activeWorkspaceId).toBe(created.id);
      expect(storeMock.set).toHaveBeenCalledWith('workspaces', expect.any(String));
      expect(storeMock.set).toHaveBeenCalledWith('active_workspace', created.id);
    });

    it('addWorkspace rejects invalid and duplicate names (case-insensitive)', async () => {
      await state.loadFromStorage(storeMock);
      await state.addWorkspace('Dev', captured(), storeMock);
      await expect(state.addWorkspace('dev', captured(), storeMock)).rejects.toThrow('already');
      await expect(state.addWorkspace('  ', captured(), storeMock)).rejects.toThrow('1 to 40');
      expect(state.workspaces).toHaveLength(1);
    });

    it('nextWorkspaceName picks the first free number', async () => {
      await state.loadFromStorage(storeMock);
      expect(state.nextWorkspaceName()).toBe('Workspace 1');
      await state.addWorkspace('Workspace 1', captured(), storeMock);
      await state.addWorkspace('Workspace 3', captured(), storeMock);
      expect(state.nextWorkspaceName()).toBe('Workspace 2');
    });

    it('replaceWorkspaceWindows keeps name and id and bumps updated', async () => {
      await state.loadFromStorage(storeMock);
      const created = await state.addWorkspace('Dev', captured(2), storeMock);
      const updated = await state.replaceWorkspaceWindows(created.id, captured(3), storeMock);
      expect(updated?.name).toBe('Dev');
      expect(updated?.id).toBe(created.id);
      expect(updated?.windows).toHaveLength(3);
      expect(updated?.link).toBeNull();
      expect(updated!.updated).toBeGreaterThanOrEqual(created.updated);
      expect(await state.replaceWorkspaceWindows('missing', captured(), storeMock)).toBeUndefined();
    });

    it('renameWorkspace renames, allows a case change, and rejects clashes', async () => {
      await state.loadFromStorage(storeMock);
      const a = await state.addWorkspace('Dev', captured(), storeMock);
      await state.addWorkspace('Mail', captured(), storeMock);
      await state.renameWorkspace(a.id, 'DEV', storeMock);
      expect(state.workspaces[0].name).toBe('DEV');
      await expect(state.renameWorkspace(a.id, 'mail', storeMock)).rejects.toThrow('already');
      await expect(state.renameWorkspace(a.id, '', storeMock)).rejects.toThrow('1 to 40');
    });

    it('deleteWorkspace removes it and clears the active id when it was active', async () => {
      await state.loadFromStorage(storeMock);
      const a = await state.addWorkspace('Dev', captured(), storeMock);
      expect(state.activeWorkspaceId).toBe(a.id);
      await state.deleteWorkspace(a.id, storeMock);
      expect(state.workspaces).toEqual([]);
      expect(state.activeWorkspaceId).toBeNull();
      expect(storeMock.delete).toHaveBeenCalledWith('active_workspace');
    });
  });
});
