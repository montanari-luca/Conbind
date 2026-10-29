/** @vitest-environment jsdom */
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('../../services/log/logService', () => ({
  logService: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));
vi.mock('../../services/windowManagement/windowManagementService', () => ({
  windowManagementService: {
    getWindowBounds: vi.fn(),
    setWindowBounds: vi.fn(),
    setFullscreen: vi.fn(),
    getMonitors: vi.fn(),
    applyPreset: vi.fn(),
    resize: vi.fn(),
    minimize: vi.fn(),
    restore: vi.fn(),
    split: vi.fn(),
    unsplit: vi.fn(),
  },
}));
vi.mock('../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: {
    showHUD: vi.fn(),
    report: vi.fn(),
  },
}));
vi.mock('../../services/action/actionService.svelte', () => ({
  actionService: {
    registerAction: vi.fn(),
    unregisterAction: vi.fn(),
  },
}));
vi.mock('./state.svelte', () => ({
  windowManagementState: {
    customLayouts: [],
    workspaces: [],
    activeWorkspace: null,
    splitAxis: 'side',
    selectedIndex: 0,
    selectedLayout: null,
    setStore: vi.fn(),
    loadFromStorage: vi.fn(),
    addCustomLayout: vi.fn(),
    deleteCustomLayout: vi.fn(),
    renameCustomLayout: vi.fn(),
    setIndex: vi.fn(),
    moveSelection: vi.fn(),
    reset: vi.fn(),
  },
}));
vi.mock('./layoutLifecycle', () => ({
  applyCustomLayout: vi.fn(),
  syncLayoutToIndex: vi.fn(),
  removeLayoutFromIndex: vi.fn(),
}));
vi.mock('./workspaceLifecycle', () => ({
  openWorkspace: vi.fn(),
  saveCurrentWorkspace: vi.fn(),
  updateWorkspace: vi.fn(),
  syncWorkspaceToIndex: vi.fn(),
  describeWorkspace: vi.fn(() => '2 windows · A, B'),
}));
vi.mock('./ManageView.svelte', () => ({ default: {} }));
vi.mock('./WorkspacesView.svelte', () => ({ default: {} }));
vi.mock('./SwitchWindowsView.svelte', () => ({ default: {} }));
vi.mock('./SplitPickerView.svelte', () => ({ default: {} }));

import extension from './index';
import { windowManagementService } from '../../services/windowManagement/windowManagementService';
import { feedbackService } from '../../services/feedback/feedbackService.svelte';
import { actionService } from '../../services/action/actionService.svelte';
import { windowManagementState } from './state.svelte';
import { applyCustomLayout, syncLayoutToIndex } from './layoutLifecycle';
import {
  openWorkspace,
  saveCurrentWorkspace,
  updateWorkspace,
  syncWorkspaceToIndex,
} from './workspaceLifecycle';
import type { ExtensionContext } from 'conbind-sdk/contracts';

function makeContext(): ExtensionContext {
  return {
    getService: vi.fn().mockImplementation((name: string) => {
      if (name === 'storage')
        return { get: vi.fn(async () => null), set: vi.fn(), delete: vi.fn() };
      if (name === 'extensions')
        return { navigateToView: vi.fn(), setActiveViewActionLabel: vi.fn() };
      return null;
    }),
  } as unknown as ExtensionContext;
}

describe('WindowManagementExtension', () => {
  beforeEach(() => vi.clearAllMocks());

  describe('initialize', () => {
    it('resolves StorageService and loads state', async () => {
      const ctx = makeContext();
      await extension.initialize(ctx);
      expect(ctx.getService).toHaveBeenCalledWith('storage');
      expect(windowManagementState.loadFromStorage).toHaveBeenCalled();
    });
  });

  describe('executeCommand — layout presets', () => {
    beforeEach(async () => {
      await extension.initialize(makeContext());
      vi.mocked(windowManagementService.applyPreset).mockResolvedValue();
      vi.mocked(feedbackService.showHUD).mockResolvedValue();
    });

    it('left-half calls applyPreset and returns no-view', async () => {
      const result = await extension.executeCommand('left-half');
      expect(windowManagementService.applyPreset).toHaveBeenCalledWith('left-half');
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Left Half');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('first-fourth calls applyPreset and returns no-view', async () => {
      const result = await extension.executeCommand('first-fourth');
      expect(windowManagementService.applyPreset).toHaveBeenCalledWith('first-fourth');
      expect(feedbackService.showHUD).toHaveBeenCalledWith('First Fourth');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('last-fourth calls applyPreset and returns no-view', async () => {
      const result = await extension.executeCommand('last-fourth');
      expect(windowManagementService.applyPreset).toHaveBeenCalledWith('last-fourth');
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Last Fourth');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('next-display calls applyPreset and returns no-view', async () => {
      const result = await extension.executeCommand('next-display');
      expect(windowManagementService.applyPreset).toHaveBeenCalledWith('next-display');
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Next Display');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('previous-display calls applyPreset and returns no-view', async () => {
      const result = await extension.executeCommand('previous-display');
      expect(windowManagementService.applyPreset).toHaveBeenCalledWith('previous-display');
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Previous Display');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('reports error diagnostic when applyPreset throws', async () => {
      vi.mocked(windowManagementService.applyPreset).mockRejectedValue(
        new Error('Accessibility permission required'),
      );
      vi.mocked(feedbackService.report).mockResolvedValue();
      const result = await extension.executeCommand('left-half');
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'manual',
          severity: 'error',
          context: expect.objectContaining({
            message: expect.stringContaining('Could not apply layout'),
          }),
        }),
      );
      expect(result).toEqual({ type: 'no-view' });
    });
  });

  describe('executeCommand — save-current-layout', () => {
    beforeEach(async () => {
      await extension.initialize(makeContext());
      vi.mocked(windowManagementService.getWindowBounds).mockResolvedValue({
        x: 100,
        y: 100,
        width: 1200,
        height: 800,
      });
      vi.mocked(feedbackService.showHUD).mockResolvedValue();
    });

    it('captures window bounds, adds custom layout and returns no-view', async () => {
      const result = await extension.executeCommand('save-current-layout');
      expect(windowManagementService.getWindowBounds).toHaveBeenCalled();
      expect(windowManagementState.addCustomLayout).toHaveBeenCalledWith(
        '1200x800',
        { x: 100, y: 100, width: 1200, height: 800 },
        expect.anything(),
      );
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Saved "1200x800"');
      expect(result).toEqual({ type: 'no-view' });
    });
  });

  describe('executeCommand — restore', () => {
    beforeEach(async () => {
      await extension.initialize(makeContext());
    });

    it('asks the backend to restore and returns no-view', async () => {
      vi.mocked(windowManagementService.restore).mockResolvedValue();
      vi.mocked(feedbackService.showHUD).mockResolvedValue();
      const result = await extension.executeCommand('restore');
      expect(windowManagementService.restore).toHaveBeenCalled();
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Restored');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('reports a friendly message when there is nothing to restore', async () => {
      vi.mocked(windowManagementService.restore).mockRejectedValue(
        new Error('window_management_restore: Not found: Nothing to restore'),
      );
      vi.mocked(feedbackService.report).mockResolvedValue();
      const result = await extension.executeCommand('restore');
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'manual',
          severity: 'error',
          context: expect.objectContaining({
            message: expect.stringContaining('Nothing to restore'),
          }),
        }),
      );
      expect(result).toEqual({ type: 'no-view' });
    });

    it('reports other restore failures with their detail', async () => {
      vi.mocked(windowManagementService.restore).mockRejectedValue(new Error('AX denied'));
      vi.mocked(feedbackService.report).mockResolvedValue();
      await extension.executeCommand('restore');
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          context: expect.objectContaining({ message: expect.stringContaining('AX denied') }),
        }),
      );
    });
  });

  describe('executeCommand — larger / smaller / minimize', () => {
    beforeEach(async () => {
      await extension.initialize(makeContext());
      vi.mocked(feedbackService.showHUD).mockResolvedValue();
      vi.mocked(feedbackService.report).mockResolvedValue();
    });

    it('larger grows the window by 10%', async () => {
      vi.mocked(windowManagementService.resize).mockResolvedValue();
      const result = await extension.executeCommand('larger');
      expect(windowManagementService.resize).toHaveBeenCalledWith(1.1);
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Larger');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('smaller shrinks the window by 10%', async () => {
      vi.mocked(windowManagementService.resize).mockResolvedValue();
      await extension.executeCommand('smaller');
      expect(windowManagementService.resize).toHaveBeenCalledWith(0.9);
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Smaller');
    });

    it('reports resize failures', async () => {
      vi.mocked(windowManagementService.resize).mockRejectedValue(new Error('no window'));
      await extension.executeCommand('larger');
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          context: expect.objectContaining({
            message: expect.stringContaining('Could not resize window'),
          }),
        }),
      );
    });

    it('minimize minimizes the frontmost window', async () => {
      vi.mocked(windowManagementService.minimize).mockResolvedValue();
      const result = await extension.executeCommand('minimize');
      expect(windowManagementService.minimize).toHaveBeenCalled();
      expect(result).toEqual({ type: 'no-view' });
    });

    it('reports minimize failures', async () => {
      vi.mocked(windowManagementService.minimize).mockRejectedValue(new Error('no window'));
      await extension.executeCommand('minimize');
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          context: expect.objectContaining({
            message: expect.stringContaining('Could not minimize window'),
          }),
        }),
      );
    });
  });

  describe('executeCommand — linked split', () => {
    let navigateToView: ReturnType<typeof vi.fn>;

    beforeEach(async () => {
      navigateToView = vi.fn();
      await extension.initialize({
        getService: vi.fn().mockImplementation((name: string) => {
          if (name === 'storage')
            return { get: vi.fn(async () => null), set: vi.fn(), delete: vi.fn() };
          if (name === 'extensions') return { navigateToView, setActiveViewActionLabel: vi.fn() };
          return null;
        }),
      } as unknown as ExtensionContext);
      vi.mocked(feedbackService.showHUD).mockResolvedValue();
      vi.mocked(feedbackService.report).mockResolvedValue();
    });

    it('split-with-window opens the picker for a side-by-side split', async () => {
      const result = await extension.executeCommand('split-with-window');
      expect(windowManagementState.splitAxis).toBe('side');
      expect(navigateToView).toHaveBeenCalledWith('window-management/SplitPickerView');
      expect(result).toMatchObject({ type: 'view', viewPath: 'window-management/SplitPickerView' });
    });

    it('split-stacked opens the picker for a stacked split', async () => {
      await extension.executeCommand('split-stacked');
      expect(windowManagementState.splitAxis).toBe('stack');
      expect(navigateToView).toHaveBeenCalledWith('window-management/SplitPickerView');
    });

    it('unlink-split reports when a split was ended', async () => {
      vi.mocked(windowManagementService.unsplit).mockResolvedValue(true);
      const result = await extension.executeCommand('unlink-split');
      expect(feedbackService.showHUD).toHaveBeenCalledWith('Split unlinked');
      expect(result).toEqual({ type: 'no-view' });
    });

    it('unlink-split says so when nothing was linked', async () => {
      vi.mocked(windowManagementService.unsplit).mockResolvedValue(false);
      await extension.executeCommand('unlink-split');
      expect(feedbackService.showHUD).toHaveBeenCalledWith('No linked split');
    });

    it('unlink-split reports failures', async () => {
      vi.mocked(windowManagementService.unsplit).mockRejectedValue(new Error('boom'));
      await extension.executeCommand('unlink-split');
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          context: expect.objectContaining({
            message: expect.stringContaining('Could not unlink split'),
          }),
        }),
      );
    });
  });

  describe('executeCommand — workspaces', () => {
    let navigateToView: ReturnType<typeof vi.fn>;
    const workspace = {
      id: 'ws-1',
      name: 'Dev',
      windows: [],
      link: null,
      created: 1,
      updated: 1,
    };

    beforeEach(async () => {
      navigateToView = vi.fn();
      await extension.initialize({
        getService: vi.fn().mockImplementation((name: string) => {
          if (name === 'storage')
            return { get: vi.fn(async () => null), set: vi.fn(), delete: vi.fn() };
          if (name === 'extensions') return { navigateToView, setActiveViewActionLabel: vi.fn() };
          return null;
        }),
      } as unknown as ExtensionContext);
      vi.mocked(feedbackService.report).mockResolvedValue();
      Object.assign(windowManagementState, { workspaces: [workspace], activeWorkspace: null });
    });

    it('save-workspace saves the current windows', async () => {
      const result = await extension.executeCommand('save-workspace');
      expect(saveCurrentWorkspace).toHaveBeenCalled();
      expect(result).toEqual({ type: 'no-view' });
    });

    it('open-workspace opens by name, ignoring case', async () => {
      await extension.executeCommand('open-workspace', { name: 'dev' });
      expect(openWorkspace).toHaveBeenCalledWith(workspace, expect.anything());
    });

    it('open-workspace opens by id', async () => {
      await extension.executeCommand('open-workspace', { id: 'ws-1' });
      expect(openWorkspace).toHaveBeenCalledWith(workspace, expect.anything());
    });

    it('open-workspace without a name or id reports an error', async () => {
      await extension.executeCommand('open-workspace');
      expect(openWorkspace).not.toHaveBeenCalled();
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          context: expect.objectContaining({
            message: expect.stringContaining('No workspace name or ID'),
          }),
        }),
      );
    });

    it('open-workspace for an unknown name reports it', async () => {
      await extension.executeCommand('open-workspace', { name: 'Nope' });
      expect(openWorkspace).not.toHaveBeenCalled();
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          context: expect.objectContaining({ message: expect.stringContaining('"Nope"') }),
        }),
      );
    });

    it('update-workspace defaults to the active workspace', async () => {
      Object.assign(windowManagementState, { activeWorkspace: workspace });
      await extension.executeCommand('update-workspace');
      expect(updateWorkspace).toHaveBeenCalledWith(workspace, expect.anything());
    });

    it('update-workspace with no name and no active workspace reports an error', async () => {
      await extension.executeCommand('update-workspace');
      expect(updateWorkspace).not.toHaveBeenCalled();
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          context: expect.objectContaining({
            message: expect.stringContaining('No workspace to update'),
          }),
        }),
      );
    });

    it('update-workspace by name updates that workspace', async () => {
      await extension.executeCommand('update-workspace', { name: 'Dev' });
      expect(updateWorkspace).toHaveBeenCalledWith(workspace, expect.anything());
    });

    it('manage-workspaces opens the workspaces view', async () => {
      const result = await extension.executeCommand('manage-workspaces');
      expect(navigateToView).toHaveBeenCalledWith('window-management/WorkspacesView');
      expect(result).toMatchObject({ type: 'view', viewPath: 'window-management/WorkspacesView' });
    });

    it('indexes saved workspaces on initialize', async () => {
      expect(syncWorkspaceToIndex).toHaveBeenCalledWith(workspace, expect.anything());
    });

    it('lists matching workspaces as search rows that open them', async () => {
      const rows = await extension.searchRows('de');
      expect(rows).toEqual([
        expect.objectContaining({
          id: 'cmd_window-management_workspace_ws-1',
          title: 'Dev',
          actionPayload: { workspaceId: 'ws-1' },
        }),
      ]);
      expect(await extension.searchRows('zzz')).toEqual([]);

      await extension.executeSearchResult('cmd_window-management_workspace_ws-1', {
        workspaceId: 'ws-1',
      });
      expect(openWorkspace).toHaveBeenCalledWith(workspace, expect.anything());
    });
  });

  describe('executeCommand — manage-layouts', () => {
    it('navigates to ManageView and returns view type', async () => {
      const ctx = makeContext();
      await extension.initialize(ctx);
      const result = await extension.executeCommand('manage-layouts');
      expect(result).toMatchObject({ type: 'view', viewPath: 'window-management/ManageView' });
    });
  });

  describe('executeCommand — switch-windows', () => {
    it('navigates to SwitchWindowsView and returns view type', async () => {
      const ctx = makeContext();
      await extension.initialize(ctx);
      const result = await extension.executeCommand('switch-windows');
      expect(result).toMatchObject({
        type: 'view',
        viewPath: 'window-management/SwitchWindowsView',
      });
    });
  });

  describe('executeCommand — apply-layout', () => {
    const layout = {
      id: 'videocall-id',
      name: 'Videocall',
      bounds: { x: 100, y: 100, width: 600, height: 400 },
    };

    beforeEach(async () => {
      await extension.initialize(makeContext());
      Object.defineProperty(windowManagementState, 'customLayouts', {
        value: [layout],
        configurable: true,
      });
    });

    it('applies layout by name case-insensitively', async () => {
      const result = await extension.executeCommand('apply-layout', { name: 'videocall' });

      expect(applyCustomLayout).toHaveBeenCalledWith(layout, expect.anything());
      expect(result).toEqual({ type: 'no-view' });
    });

    it('applies layout by id', async () => {
      const result = await extension.executeCommand('apply-layout', { id: 'videocall-id' });

      expect(applyCustomLayout).toHaveBeenCalledWith(layout, expect.anything());
      expect(result).toEqual({ type: 'no-view' });
    });

    it('reports error when layout is not found', async () => {
      vi.mocked(feedbackService.report).mockResolvedValue();

      const result = await extension.executeCommand('apply-layout', { name: 'Nonexistent' });

      expect(feedbackService.showHUD).toHaveBeenCalledWith(expect.stringContaining('Nonexistent'), {
        severity: 'warning',
      });
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'manual',
          severity: 'error',
          context: expect.objectContaining({
            message: expect.stringContaining('Nonexistent'),
          }),
        }),
      );
      expect(result).toEqual({ type: 'no-view' });
    });

    it('reports error when neither name nor id is provided', async () => {
      vi.mocked(feedbackService.report).mockResolvedValue();

      const result = await extension.executeCommand('apply-layout', {});

      expect(feedbackService.showHUD).toHaveBeenCalledWith('No layout name or ID provided', {
        severity: 'warning',
      });
      expect(feedbackService.report).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'manual',
          severity: 'error',
          context: expect.objectContaining({
            message: expect.stringContaining('No layout name or ID provided'),
          }),
        }),
      );
      expect(result).toEqual({ type: 'no-view' });
    });
  });

  describe('search', () => {
    it('returns custom layouts as ExtensionResult entries', async () => {
      const layout = {
        id: '1',
        name: 'My Layout',
        bounds: { x: 0, y: 0, width: 800, height: 600 },
      };
      Object.defineProperty(windowManagementState, 'customLayouts', {
        value: [layout],
        configurable: true,
      });
      await extension.initialize(makeContext());
      const results = await extension.searchRows('my');
      expect(results.length).toBeGreaterThan(0);
      expect(results[0].title).toContain('My Layout');
      expect(results[0].id).toBe('cmd_window-management_layout_1');
    });

    it('returns empty array when no custom layouts match', async () => {
      Object.defineProperty(windowManagementState, 'customLayouts', {
        value: [],
        configurable: true,
      });
      await extension.initialize(makeContext());
      const results = await extension.searchRows('anything');
      expect(results).toEqual([]);
    });
  });

  describe('lifecycle (activate and deactivate)', () => {
    it('unregisters manage actions and resets state on deactivate', async () => {
      await extension.initialize(makeContext());
      await extension.viewActivated('window-management/ManageView');
      expect(actionService.registerAction).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'window-management:save-current-window' }),
      );

      await extension.deactivate();
      expect(actionService.unregisterAction).toHaveBeenCalledWith(
        'window-management:save-current-window',
      );
      expect(windowManagementState.reset).toHaveBeenCalled();
    });

    it('reloads from storage and syncs custom layouts to index on activate', async () => {
      const layout = {
        id: '1',
        name: 'My Layout',
        bounds: { x: 0, y: 0, width: 800, height: 600 },
      };
      Object.defineProperty(windowManagementState, 'customLayouts', {
        value: [layout],
        configurable: true,
      });

      await extension.initialize(makeContext());
      await extension.activate();

      expect(windowManagementState.loadFromStorage).toHaveBeenCalled();
      expect(syncLayoutToIndex).toHaveBeenCalled();
    });
  });
});

it('registers window-management as a typed platform search provider', async () => {
  const { searchBuiltinProviders } = await import('../../services/search/builtinSearchProviders');
  const search = vi.spyOn(extension, 'searchRows').mockResolvedValueOnce([]);
  const rows = await searchBuiltinProviders('', (id) => id === 'window-management');
  expect(search).toHaveBeenCalled();
  search.mockRestore();
  expect(rows.every((row) => typeof (row as any).action !== 'function')).toBe(true);
});
