import { t } from '../../services/i18n';
import { windowManagementState, type Workspace } from './state.svelte';
import { windowManagementService } from '../../services/windowManagement/windowManagementService';
import { feedbackService } from '../../services/feedback/feedbackService.svelte';
import { searchService } from '../../services/search/SearchService';
import { commandService } from '../../services/extension/commandService.svelte';
import { shortcutService } from '../shortcuts/shortcutService';
import { logService } from '../../services/log/logService';
import type { IStorageService } from 'conbind-sdk/contracts';

const objectIdFor = (workspaceId: string) => `cmd_window-management_workspace_${workspaceId}`;

async function reportFailure(err: any, key: string, detailKey: string): Promise<void> {
  await feedbackService.report({
    source: 'frontend',
    kind: 'manual',
    severity: 'error',
    retryable: false,
    context: {
      message: err?.message ? t(detailKey, { detail: err.message }) : t(key),
    },
  });
}

/** "3 windows · Safari, Notes, Slack" */
export function describeWorkspace(workspace: Workspace): string {
  const apps = [...new Set(workspace.windows.map((w) => w.appName))].join(', ');
  const count = t(
    workspace.windows.length === 1
      ? 'features.window_management.workspace_count_one'
      : 'features.window_management.workspace_count_many',
    { count: workspace.windows.length },
  );
  return `${count} · ${apps}`;
}

/** Arranges the workspace's windows, launching missing apps, and makes it the active one. */
export async function openWorkspace(workspace: Workspace, store?: IStorageService): Promise<void> {
  try {
    const report = await windowManagementService.openWorkspace(workspace.windows, workspace.link);
    await windowManagementState.setActiveWorkspace(workspace.id, store);
    await feedbackService.showHUD(
      report.missing.length
        ? t('features.window_management.hud_workspace_partial', {
            name: workspace.name,
            missing: report.missing.join(', '),
          })
        : t('features.window_management.hud_workspace_opened', { name: workspace.name }),
    );
  } catch (err: any) {
    logService.error(`[WindowManagement] openWorkspace failed: ${err}`);
    await reportFailure(
      err,
      'features.window_management.err_workspace_open',
      'features.window_management.err_workspace_open_detail',
    );
  }
}

/** Saves the frontmost window and others on its display as a new "Workspace N". */
export async function saveCurrentWorkspace(store?: IStorageService): Promise<Workspace | null> {
  try {
    const captured = await windowManagementService.captureWorkspace();
    const workspace = await windowManagementState.addWorkspace(
      windowManagementState.nextWorkspaceName(),
      captured,
      store,
    );
    await syncWorkspaceToIndex(workspace, store);
    await feedbackService.showHUD(
      t('features.window_management.hud_workspace_saved', { name: workspace.name }),
    );
    return workspace;
  } catch (err: any) {
    logService.error(`[WindowManagement] saveCurrentWorkspace failed: ${err}`);
    await reportFailure(
      err,
      'features.window_management.err_workspace_save',
      'features.window_management.err_workspace_save_detail',
    );
    return null;
  }
}

/** Overwrites the workspace with the windows currently on screen. */
export async function updateWorkspace(
  workspace: Workspace,
  store?: IStorageService,
): Promise<void> {
  try {
    const captured = await windowManagementService.captureWorkspace();
    await windowManagementState.replaceWorkspaceWindows(workspace.id, captured, store);
    await feedbackService.showHUD(
      t('features.window_management.hud_workspace_updated', { name: workspace.name }),
    );
  } catch (err: any) {
    logService.error(`[WindowManagement] updateWorkspace failed: ${err}`);
    await reportFailure(
      err,
      'features.window_management.err_workspace_save',
      'features.window_management.err_workspace_save_detail',
    );
  }
}

export async function syncWorkspaceToIndex(
  workspace: Workspace,
  store?: IStorageService,
): Promise<void> {
  const objectId = objectIdFor(workspace.id);
  await searchService.indexItem({
    category: 'command',
    id: objectId,
    name: workspace.name,
    extension: 'window-management',
    trigger: `workspace ${workspace.name}`,
    type: 'command',
    icon: 'icon:window',
  });

  commandService.registerCommand(
    objectId,
    {
      execute: async () => {
        // Read the current copy: the workspace may have been updated since it was indexed.
        const current = windowManagementState.workspaces.find((w) => w.id === workspace.id);
        if (current) await openWorkspace(current, store);
        return { type: 'no-view' };
      },
    },
    'window-management',
  );
}

export async function removeWorkspaceFromIndex(workspaceId: string): Promise<void> {
  const objectId = objectIdFor(workspaceId);
  await searchService.deleteItem(objectId);
  commandService.unregisterCommand(objectId);
  await shortcutService.unregister(objectId);
}

export async function deleteWorkspace(workspaceId: string, store?: IStorageService): Promise<void> {
  await windowManagementState.deleteWorkspace(workspaceId, store);
  await removeWorkspaceFromIndex(workspaceId);
}

/** Throws when the name is invalid or already taken. */
export async function renameWorkspace(
  workspaceId: string,
  newName: string,
  store?: IStorageService,
): Promise<void> {
  await windowManagementState.renameWorkspace(workspaceId, newName, store);
  const updated = windowManagementState.workspaces.find((w) => w.id === workspaceId);
  if (updated) {
    await syncWorkspaceToIndex(updated, store);
  }
}
