<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ListItem, EmptyState, Modal, Input, Button } from '../../components';
  import { windowManagementState, MAX_WORKSPACE_NAME } from './state.svelte';
  import { feedbackService } from '../../services/feedback/feedbackService.svelte';
  import { actionService } from '../../services/action/actionService.svelte';
  import {
    describeWorkspace,
    openWorkspace,
    saveCurrentWorkspace,
    updateWorkspace,
    deleteWorkspace,
    renameWorkspace,
  } from './workspaceLifecycle';
  import { hideWindow } from '../../lib/ipc/commands';
  import { resetLauncherState } from '../../lib/launcher/launcherReset';
  import { isAnyModalOpen } from '../../components/base/Modal.logic';
  import type { IStorageService } from 'conbind-sdk/contracts';
  import { ActionContext } from 'conbind-sdk/contracts';
  import { t } from '../../services/i18n';

  interface Props {
    store?: IStorageService;
    extensionManager?: any;
  }
  let { store, extensionManager }: Props = $props();

  const activeStore = () => store ?? windowManagementState.store ?? undefined;

  let isRenameModalOpen = $state(false);
  let editingId = $state<string | null>(null);
  let editingName = $state('');

  let workspaces = $derived(windowManagementState.workspaces);
  let selectedIndex = $derived(windowManagementState.selectedWorkspaceIndex);
  let selectedWorkspace = $derived(windowManagementState.selectedWorkspace);

  async function handleOpen(id: string) {
    const ws = workspaces.find((w) => w.id === id);
    if (!ws) return;
    // Hide first: opening raises the saved windows and may take a few seconds to launch apps.
    await hideWindow();
    resetLauncherState();
    await openWorkspace(ws, activeStore());
  }

  async function handleSaveCurrent() {
    await saveCurrentWorkspace(activeStore());
  }

  async function handleUpdate(id: string) {
    const ws = workspaces.find((w) => w.id === id);
    if (!ws) return;
    await updateWorkspace(ws, activeStore());
  }

  function startRename(id: string) {
    const ws = workspaces.find((w) => w.id === id);
    if (!ws) return;
    editingId = id;
    editingName = ws.name;
    isRenameModalOpen = true;
  }

  async function commitRename() {
    if (!editingId) {
      isRenameModalOpen = false;
      return;
    }
    const id = editingId;
    const newName = editingName.trim();
    isRenameModalOpen = false;
    editingId = null;
    if (!newName) return;
    try {
      await renameWorkspace(id, newName, activeStore());
      await feedbackService.showHUD(
        t('features.window_management.hud_workspace_renamed', { name: newName }),
      );
    } catch (err: any) {
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: t('features.window_management.err_workspace_rename_detail', {
            detail: String(err?.message ?? err),
          }),
        },
      });
    }
  }

  function cancelRename() {
    isRenameModalOpen = false;
    editingId = null;
    editingName = '';
  }

  async function handleDelete(id: string) {
    const ws = workspaces.find((w) => w.id === id);
    if (!ws) return;
    const name = ws.name;
    await deleteWorkspace(id, activeStore());
    await feedbackService.showHUD(t('features.window_management.hud_workspace_deleted', { name }));
  }

  function handleKeydown(event: KeyboardEvent) {
    if (document.querySelector('.action-popup')) return;
    if (isAnyModalOpen(document)) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (workspaces.length === 0) return;

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      event.stopPropagation();
      windowManagementState.moveWorkspaceSelection(event.key === 'ArrowUp' ? 'up' : 'down');
    } else if (event.key === 'Enter' && selectedWorkspace) {
      event.preventDefault();
      event.stopPropagation();
      void handleOpen(selectedWorkspace.id);
    }
  }

  const ACTION_IDS = [
    'window-management:open-workspace',
    'window-management:update-workspace',
    'window-management:rename-workspace',
    'window-management:delete-workspace',
  ];

  $effect(() => {
    actionService.registerAction({
      id: 'window-management:save-workspace',
      title: t('features.window_management.save_workspace_title'),
      description: t('features.window_management.save_workspace_desc'),
      icon: 'icon:plus',
      shortcut: 'Mod+Shift+N',
      extensionId: 'window-management',
      category: 'window-management',
      context: ActionContext.EXTENSION_VIEW,
      execute: () => handleSaveCurrent(),
    });

    const current = selectedWorkspace;
    if (!current) {
      ACTION_IDS.forEach((id) => actionService.unregisterAction(id));
      return;
    }

    const id = current.id;
    const name = current.name;
    const base = {
      extensionId: 'window-management',
      category: 'window-management',
      context: ActionContext.EXTENSION_VIEW,
    };

    actionService.registerAction({
      ...base,
      id: ACTION_IDS[0],
      title: t('features.window_management.ui.open_workspace_title', { name }),
      icon: 'icon:play',
      execute: () => handleOpen(id),
    });
    actionService.registerAction({
      ...base,
      id: ACTION_IDS[1],
      title: t('features.window_management.ui.update_workspace_title', { name }),
      icon: 'icon:refresh',
      execute: () => handleUpdate(id),
    });
    actionService.registerAction({
      ...base,
      id: ACTION_IDS[2],
      title: t('features.window_management.ui.rename_title', { name }),
      icon: 'icon:edit',
      shortcut: 'Mod+Shift+R',
      execute: () => startRename(id),
    });
    actionService.registerAction({
      ...base,
      id: ACTION_IDS[3],
      title: t('features.window_management.ui.delete_title', { name }),
      icon: 'icon:trash',
      execute: () => handleDelete(id),
    });

    return () => ACTION_IDS.forEach((actionId) => actionService.unregisterAction(actionId));
  });

  onMount(() => {
    extensionManager?.setActiveViewActionLabel(t('features.window_management.open_action'));
    window.addEventListener('keydown', handleKeydown, true);
  });

  onDestroy(() => {
    extensionManager?.setActiveViewActionLabel(null);
    window.removeEventListener('keydown', handleKeydown, true);
    actionService.unregisterAction('window-management:save-workspace');
    ACTION_IDS.forEach((id) => actionService.unregisterAction(id));
  });
</script>

<div class="view-container">
  <div class="list custom-scrollbar">
    {#if workspaces.length === 0}
      <EmptyState
        message={t('features.window_management.no_workspaces')}
        description={t('features.window_management.no_workspaces_description')}
      />
    {:else}
      <div class="section-header">{t('features.window_management.workspaces_header')}</div>
      {#each workspaces as ws, index (ws.id)}
        <ListItem
          title={ws.name}
          subtitle={describeWorkspace(ws)}
          selected={selectedIndex === index}
          onclick={() => {
            windowManagementState.setWorkspaceIndex(index);
          }}
          ondblclick={() => {
            void handleOpen(ws.id);
          }}
        >
          {#snippet leading()}
            <div class="workspace-icon">▦</div>
          {/snippet}
        </ListItem>
      {/each}
    {/if}
  </div>
</div>

<Modal
  bind:isOpen={isRenameModalOpen}
  title={t('features.window_management.rename_workspace_title')}
  subtitle={t('features.window_management.rename_workspace_subtitle', {
    max: MAX_WORKSPACE_NAME,
  })}
  onEnter={commitRename}
  onEscape={cancelRename}
>
  <div class="modal-body">
    <Input
      placeholder={t('features.window_management.rename_workspace_placeholder')}
      bind:value={editingName}
    />
  </div>
  {#snippet actions()}
    <div class="modal-actions">
      <Button onclick={cancelRename}>{t('common.cancel')}</Button>
      <Button variant="primary" onclick={commitRename} disabled={!editingName.trim()}
        >{t('common.save')}</Button
      >
    </div>
  {/snippet}
</Modal>

<style>
  .view-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .section-header {
    padding: var(--space-2) var(--space-5) var(--space-1);
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }

  .workspace-icon {
    width: var(--size-xl);
    height: var(--size-xl);
    border-radius: var(--radius-md);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: var(--font-size-lg);
  }

  .modal-body {
    padding: var(--space-4) 0;
  }

  .modal-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
  }
</style>
