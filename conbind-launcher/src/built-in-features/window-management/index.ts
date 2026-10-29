import { t } from '../../services/i18n';
import {
  registerBuiltinSearchProvider,
  type BuiltinSearchRow,
} from '../../services/search/builtinSearchProviders';
import { logService } from '../../services/log/logService';
import { windowManagementService } from '../../services/windowManagement/windowManagementService';
import { feedbackService } from '../../services/feedback/feedbackService.svelte';
import { actionService } from '../../services/action/actionService.svelte';
import { windowManagementState } from './state.svelte';
import { DISPLAY_COMMAND_IDS, PRESET_IDS } from './presets';
import { extractErrorMessage } from '../../lib/errors';
import { applyCustomLayout, syncLayoutToIndex, removeLayoutFromIndex } from './layoutLifecycle';
import {
  describeWorkspace,
  openWorkspace,
  saveCurrentWorkspace,
  syncWorkspaceToIndex,
  updateWorkspace,
} from './workspaceLifecycle';
import { isAnyModalOpen } from '../../components/base/Modal.logic';
import ManageView from './ManageView.svelte';
import SwitchWindowsView from './SwitchWindowsView.svelte';
import SplitPickerView from './SplitPickerView.svelte';
import WorkspacesView from './WorkspacesView.svelte';
import {
  type Extension,
  type ExtensionContext,
  type IStorageService,
  type IExtensionManager,
  ActionContext,
} from 'conbind-sdk/contracts';

const RESIZE_STEP = 0.1;

class WindowManagementExtension implements Extension {
  onUnload: any;

  private store?: IStorageService;
  private extensionManager?: IExtensionManager;
  private inView = false;

  async initialize(context: ExtensionContext): Promise<void> {
    this.store = context.getService<IStorageService>('storage');
    this.extensionManager = context.getService<IExtensionManager>('extensions');
    if (this.store) {
      windowManagementState.setStore(this.store);
      await windowManagementState.loadFromStorage(this.store);
      await this.syncIndex();
    }
    logService.info('[WindowManagement] Initialized');
  }

  /** Makes every saved layout and workspace searchable and bindable to a hotkey. */
  private async syncIndex(): Promise<void> {
    if (!this.store) return;
    for (const layout of windowManagementState.customLayouts) {
      await syncLayoutToIndex(layout, this.store);
    }
    for (const workspace of windowManagementState.workspaces) {
      await syncWorkspaceToIndex(workspace, this.store);
    }
  }

  async executeCommand(commandId: string, _args?: Record<string, any>): Promise<any> {
    if (commandId === 'restore') {
      await this.restorePreviousBounds();
      return { type: 'no-view' };
    }

    if (commandId === 'larger' || commandId === 'smaller') {
      await this.resize(commandId === 'larger' ? 1 + RESIZE_STEP : 1 - RESIZE_STEP, commandId);
      return { type: 'no-view' };
    }

    if (commandId === 'minimize') {
      await this.minimize();
      return { type: 'no-view' };
    }

    if (commandId === 'split-with-window' || commandId === 'split-stacked') {
      windowManagementState.splitAxis = commandId === 'split-stacked' ? 'stack' : 'side';
      this.extensionManager?.navigateToView('window-management/SplitPickerView');
      return { type: 'view', viewPath: 'window-management/SplitPickerView' };
    }

    if (commandId === 'unlink-split') {
      await this.unlinkSplit();
      return { type: 'no-view' };
    }

    if (commandId === 'save-workspace') {
      await saveCurrentWorkspace(this.store);
      return { type: 'no-view' };
    }

    if (commandId === 'open-workspace' || commandId === 'update-workspace') {
      await this.runWorkspaceCommand(commandId, _args);
      return { type: 'no-view' };
    }

    if (commandId === 'manage-workspaces') {
      this.extensionManager?.navigateToView('window-management/WorkspacesView');
      return { type: 'view', viewPath: 'window-management/WorkspacesView' };
    }

    if (commandId === 'manage-layouts') {
      this.extensionManager?.navigateToView('window-management/ManageView');
      return { type: 'view', viewPath: 'window-management/ManageView' };
    }

    if (commandId === 'switch-windows') {
      this.extensionManager?.navigateToView('window-management/SwitchWindowsView');
      return { type: 'view', viewPath: 'window-management/SwitchWindowsView' };
    }

    if (commandId === 'save-current-layout') {
      await this.saveCurrentWindowLayout();
      return { type: 'no-view' };
    }

    if (commandId === 'apply-layout') {
      await this.applyLayoutByArgs(_args);
      return { type: 'no-view' };
    }

    if (
      (PRESET_IDS as readonly string[]).includes(commandId) ||
      (DISPLAY_COMMAND_IDS as readonly string[]).includes(commandId)
    ) {
      await this.applyPreset(commandId);
      return { type: 'no-view' };
    }

    logService.warn(`[WindowManagement] Unknown command: ${commandId}`);
    return { type: 'no-view' };
  }

  /** `open-workspace` needs a name or id; `update-workspace` falls back to the active workspace. */
  private async runWorkspaceCommand(
    commandId: 'open-workspace' | 'update-workspace',
    args?: Record<string, any>,
  ): Promise<void> {
    const rawName = typeof args?.name === 'string' ? args.name.trim() : undefined;
    const rawId = typeof args?.id === 'string' ? args.id.trim() : undefined;

    if (!rawName && !rawId && commandId === 'open-workspace') {
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: { message: t('features.window_management.err_no_workspace') },
      });
      return;
    }

    const workspace =
      rawName || rawId
        ? windowManagementState.workspaces.find(
            (w) =>
              (rawId && w.id === rawId) ||
              (rawName && w.name.toLowerCase() === rawName.toLowerCase()),
          )
        : windowManagementState.activeWorkspace;

    if (!workspace) {
      const missing = rawName || rawId;
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: missing
            ? t('features.window_management.err_workspace_not_found', { name: missing })
            : t('features.window_management.err_no_active_workspace'),
        },
      });
      return;
    }

    if (commandId === 'open-workspace') {
      await openWorkspace(workspace, this.store);
    } else {
      await updateWorkspace(workspace, this.store);
    }
  }

  private async applyLayoutByArgs(args?: Record<string, any>): Promise<void> {
    const rawName = typeof args?.name === 'string' ? args.name.trim() : undefined;
    const rawId = typeof args?.id === 'string' ? args.id.trim() : undefined;

    if (!rawName && !rawId) {
      await feedbackService.showHUD('No layout name or ID provided', { severity: 'warning' });
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: { message: t('features.window_management.err_no_layout') },
      });
      return;
    }

    const targetName = rawName?.toLowerCase();
    const layout = windowManagementState.customLayouts.find(
      (l) => (rawId && l.id === rawId) || (targetName && l.name.toLowerCase() === targetName),
    );

    if (layout) {
      await applyCustomLayout(layout, this.store);
    } else {
      const missing = rawName || rawId || 'unknown';
      await feedbackService.showHUD(`Layout "${missing}" not found`, { severity: 'warning' });
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: { message: t('features.window_management.err_not_found', { name: missing }) },
      });
    }
  }

  private async applyPreset(presetId: string): Promise<void> {
    try {
      await windowManagementService.applyPreset(presetId);

      const label = presetId
        .split('-')
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
        .join(' ');
      await feedbackService.showHUD(label);
    } catch (err: any) {
      logService.error(`[WindowManagement] applyPreset failed: ${err}`);
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: err.message
            ? t('features.window_management.err_apply_detail', { detail: err.message })
            : t('features.window_management.err_apply'),
        },
      });
    }
  }

  private async resize(factor: number, commandId: 'larger' | 'smaller'): Promise<void> {
    try {
      await windowManagementService.resize(factor);
      await feedbackService.showHUD(
        t(
          commandId === 'larger'
            ? 'features.window_management.hud_larger'
            : 'features.window_management.hud_smaller',
        ),
      );
    } catch (err: any) {
      logService.error(`[WindowManagement] resize failed: ${err}`);
      await this.reportFailure(
        err,
        'features.window_management.err_resize',
        'features.window_management.err_resize_detail',
      );
    }
  }

  private async minimize(): Promise<void> {
    try {
      await windowManagementService.minimize();
    } catch (err: any) {
      logService.error(`[WindowManagement] minimize failed: ${err}`);
      await this.reportFailure(
        err,
        'features.window_management.err_minimize',
        'features.window_management.err_minimize_detail',
      );
    }
  }

  private async unlinkSplit(): Promise<void> {
    try {
      const wasLinked = await windowManagementService.unsplit();
      await feedbackService.showHUD(
        t(
          wasLinked
            ? 'features.window_management.hud_unlinked'
            : 'features.window_management.hud_no_split',
        ),
      );
    } catch (err: any) {
      logService.error(`[WindowManagement] unsplit failed: ${err}`);
      await this.reportFailure(
        err,
        'features.window_management.err_unsplit',
        'features.window_management.err_unsplit_detail',
      );
    }
  }

  private async restorePreviousBounds(): Promise<void> {
    try {
      await windowManagementService.restore();
      await feedbackService.showHUD(t('features.window_management.hud_restored'));
    } catch (err: any) {
      const detail = extractErrorMessage(err);
      if (detail.includes('Nothing to restore')) {
        await this.reportFailure(null, 'features.window_management.nothing_restore');
        return;
      }
      await this.reportFailure(
        err,
        'features.window_management.err_restore',
        'features.window_management.err_restore_detail',
      );
    }
  }

  private async reportFailure(err: any, fallbackKey: string, detailKey?: string): Promise<void> {
    await feedbackService.report({
      source: 'frontend',
      kind: 'manual',
      severity: 'error',
      retryable: false,
      context: {
        message: err?.message && detailKey ? t(detailKey, { detail: err.message }) : t(fallbackKey),
      },
    });
  }

  async searchRows(query: string): Promise<BuiltinSearchRow[]> {
    const { customLayouts, workspaces } = windowManagementState;
    const q = query.toLowerCase();

    const layoutRows: BuiltinSearchRow[] = customLayouts
      .filter((l) => l.name.toLowerCase().includes(q))
      .map((layout) => ({
        id: `cmd_window-management_layout_${layout.id}`,
        title: layout.name,
        subtitle: t('features.window_management.layout_subtitle', {
          w: Math.round(layout.bounds.width),
          h: Math.round(layout.bounds.height),
          x: Math.round(layout.bounds.x),
          y: Math.round(layout.bounds.y),
        }),
        score: 0.7,
        icon: 'icon:store',
        actionPayload: { layoutId: layout.id },
      }));

    const workspaceRows: BuiltinSearchRow[] = workspaces
      .filter((w) => w.name.toLowerCase().includes(q))
      .map((workspace) => ({
        id: `cmd_window-management_workspace_${workspace.id}`,
        title: workspace.name,
        subtitle: describeWorkspace(workspace),
        score: 0.7,
        icon: 'icon:window',
        actionPayload: { workspaceId: workspace.id },
      }));

    return [...layoutRows, ...workspaceRows];
  }

  async executeSearchResult(_id: string, payload?: unknown): Promise<void> {
    const { layoutId, workspaceId } = (payload ?? {}) as {
      layoutId?: unknown;
      workspaceId?: unknown;
    };
    if (workspaceId !== undefined) {
      const workspace = windowManagementState.workspaces.find((w) => w.id === workspaceId);
      if (workspace) await openWorkspace(workspace, this.store);
      return;
    }
    const layout = windowManagementState.customLayouts.find((layout) => layout.id === layoutId);
    if (layout) await applyCustomLayout(layout, this.store);
  }

  private handleKeydownBound = (event: KeyboardEvent) => this.handleKeydown(event);

  async viewActivated(viewPath: string): Promise<void> {
    this.inView = true;
    if (viewPath === 'window-management/ManageView') {
      if (typeof window !== 'undefined') {
        window.removeEventListener('keydown', this.handleKeydownBound);
        window.addEventListener('keydown', this.handleKeydownBound);
      }
      this.registerManageActions();
      this.extensionManager?.setActiveViewActionLabel('Apply');
    }
    logService.debug(`[WindowManagement] View activated: ${viewPath}`);
  }

  async viewDeactivated(viewPath: string): Promise<void> {
    this.inView = false;
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', this.handleKeydownBound);
    }
    this.unregisterManageActions();
    logService.debug(`[WindowManagement] View deactivated: ${viewPath}`);
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (!this.inView) return;
    if (isAnyModalOpen(document)) return;
    const layouts = windowManagementState.customLayouts;
    if (!layouts.length) return;

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      event.stopPropagation();
      windowManagementState.moveSelection(event.key === 'ArrowUp' ? 'up' : 'down');
    } else if (event.key === 'Enter') {
      const selected = windowManagementState.selectedLayout;
      if (selected) {
        event.preventDefault();
        event.stopPropagation();
        void applyCustomLayout(selected, this.store);
      }
    }
  }

  private registerManageActions(): void {
    actionService.registerAction({
      id: 'window-management:save-current-window',
      title: t('features.window_management.act.save_layout'),
      description: t('features.window_management.act.save_layout_desc'),
      icon: 'icon:plus',
      extensionId: 'window-management',
      category: 'window-management',
      context: ActionContext.EXTENSION_VIEW,
      execute: () => this.saveCurrentWindowLayout(),
    });
  }

  private unregisterManageActions(): void {
    actionService.unregisterAction('window-management:save-current-window');
  }

  private async saveCurrentWindowLayout(): Promise<void> {
    if (!this.store) return;
    try {
      const bounds = await windowManagementService.getWindowBounds();
      const name = `${Math.round(bounds.width)}x${Math.round(bounds.height)}`;
      await windowManagementState.addCustomLayout(name, bounds, this.store);
      const created =
        windowManagementState.customLayouts.find(
          (l) => l.name === name && l.bounds.x === bounds.x && l.bounds.y === bounds.y,
        ) ?? windowManagementState.customLayouts[windowManagementState.customLayouts.length - 1];
      if (created) {
        await syncLayoutToIndex(created, this.store);
      }
      await feedbackService.showHUD(`Saved "${name}"`);
    } catch (err: any) {
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: err.message
            ? t('features.window_management.err_save_detail', { detail: err.message })
            : t('features.window_management.err_save'),
        },
      });
    }
  }

  async activate(): Promise<void> {
    if (this.store) {
      await windowManagementState.loadFromStorage(this.store);
      await this.syncIndex();
    }
  }

  async deactivate(): Promise<void> {
    this.inView = false;
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', this.handleKeydownBound);
    }
    this.unregisterManageActions();
    windowManagementState.reset();
  }
}

const extension = new WindowManagementExtension();
registerBuiltinSearchProvider({
  extensionId: 'window-management',
  search: (query) => extension.searchRows(query),
  execute: (id, payload) => extension.executeSearchResult(id, payload),
});
export default extension;
export { ManageView, SwitchWindowsView, SplitPickerView, WorkspacesView };
