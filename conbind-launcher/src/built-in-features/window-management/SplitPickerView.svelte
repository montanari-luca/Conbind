<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import type { AppWindowInfo } from '../../bindings';
  import { windowManagementService } from '../../services/windowManagement/windowManagementService';
  import { feedbackService } from '../../services/feedback/feedbackService.svelte';
  import { searchStores } from '../../services/search/stores/search.svelte';
  import { logService } from '../../services/log/logService';
  import { t } from '../../services/i18n';
  import { isAnyModalOpen } from '../../components/base/Modal.logic';
  import { LauncherListRow, EmptyState, LoadingState } from '../../components';
  import { hideWindow } from '../../lib/ipc/commands';
  import { resetLauncherState } from '../../lib/launcher/launcherReset';
  import { windowManagementState } from './state.svelte';

  interface Props {
    extensionManager?: any;
  }
  let { extensionManager }: Props = $props();

  let windows = $state<AppWindowInfo[]>([]);
  let isLoading = $state(true);
  let selectedIndex = $state(0);
  let listContainer = $state<HTMLElement | null>(null);

  const query = $derived(searchStores.query.trim().toLowerCase());
  // The frontmost window is the other half of the pair, and a minimized one can't be tiled.
  const candidates = $derived(windows.filter((w) => !w.isFocused && !w.isMinimized));
  const filteredWindows = $derived.by(() => {
    if (!query) return candidates;
    return candidates.filter(
      (w) => w.title.toLowerCase().includes(query) || w.appName.toLowerCase().includes(query),
    );
  });
  const selectedWindow = $derived(filteredWindows[selectedIndex] ?? null);

  $effect(() => {
    if (filteredWindows.length === 0) {
      selectedIndex = 0;
    } else if (selectedIndex >= filteredWindows.length) {
      selectedIndex = filteredWindows.length - 1;
    }
  });

  async function refreshWindows() {
    try {
      isLoading = true;
      windows = await windowManagementService.listWindows();
    } catch (err: any) {
      logService.error(`[SplitPickerView] listWindows failed: ${err}`);
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: t('features.window_management.ui.list_failed', {
            error: String(err?.message ?? err),
          }),
        },
      });
    } finally {
      isLoading = false;
    }
  }

  async function handleSplit(id: string) {
    try {
      await windowManagementService.split(id, windowManagementState.splitAxis);
      await feedbackService.showHUD(t('features.window_management.hud_split'));
      await hideWindow();
      resetLauncherState();
    } catch (err: any) {
      logService.error(`[SplitPickerView] split failed: ${err}`);
      await feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: t('features.window_management.ui.split_failed', {
            error: String(err?.message ?? err),
          }),
        },
      });
    }
  }

  function scrollToSelected() {
    tick().then(() => {
      const selectedEl = listContainer?.querySelector('.selected-result');
      if (typeof selectedEl?.scrollIntoView === 'function') {
        selectedEl.scrollIntoView({ block: 'nearest' });
      }
    });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (document.querySelector('.action-popup')) return;
    if (isAnyModalOpen(document)) return;
    if (filteredWindows.length === 0) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      event.stopPropagation();
      selectedIndex = (selectedIndex + 1) % filteredWindows.length;
      scrollToSelected();
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      event.stopPropagation();
      selectedIndex = (selectedIndex - 1 + filteredWindows.length) % filteredWindows.length;
      scrollToSelected();
    } else if (event.key === 'Enter') {
      if (selectedWindow) {
        event.preventDefault();
        event.stopPropagation();
        void handleSplit(selectedWindow.id);
      }
    }
  }

  onMount(() => {
    extensionManager?.setActiveViewActionLabel(t('features.window_management.split_action'));
    void refreshWindows();
    window.addEventListener('keydown', handleKeydown, true);
  });

  onDestroy(() => {
    extensionManager?.setActiveViewActionLabel(null);
    window.removeEventListener('keydown', handleKeydown, true);
  });
</script>

<div class="view-container">
  <div class="list custom-scrollbar" bind:this={listContainer}>
    {#if isLoading && windows.length === 0}
      <LoadingState />
    {:else if candidates.length === 0}
      <EmptyState message={t('features.window_management.no_split_windows')} description="" />
    {:else if filteredWindows.length === 0}
      <EmptyState message={t('features.window_management.no_matching_windows')} description="" />
    {:else}
      {#each filteredWindows as win, index (win.id)}
        <LauncherListRow
          title={win.title || win.appName}
          subtitle={win.appName}
          icon={win.appIcon ?? 'icon:window'}
          selected={selectedIndex === index}
          onclick={() => {
            selectedIndex = index;
            void handleSplit(win.id);
          }}
        />
      {/each}
    {/if}
  </div>
</div>

<style>
  .view-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }
</style>
