<script lang="ts">
  import { AppBar, EmptyState, Icon, LoadingState, MeterBar } from '../../components';
  import { formatBytes } from '../../services/action/actionService.svelte';
  import { t } from '../../services/i18n';
  import { scrollSelectedIntoView } from '../../lib/listScroll';
  import FullDiskAccessBanner from './FullDiskAccessBanner.svelte';
  import { analyzerState } from './analyzerState.svelte';
  import { entryFraction } from './analyzerLogic';

  let listEl = $state<HTMLDivElement | undefined>();
  let dir = $derived(analyzerState.current);

  $effect(() => {
    const index = analyzerState.cursor;
    if (listEl) scrollSelectedIntoView(listEl, index);
  });
</script>

<div class="analyzer-view">
  <AppBar title={t('features.cleanup.ui.analyzer_title')} />
  <FullDiskAccessBanner />

  {#if dir}
    <div class="crumb">
      <div class="crumb-path truncate">{dir.path}</div>
      <div class="crumb-total">{formatBytes(dir.totalBytes)}</div>
    </div>
  {/if}

  <div class="analyzer-body custom-scrollbar" bind:this={listEl} role="listbox">
    {#if dir}
      {#each dir.entries as entry, i (entry.path)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="row"
          class:selected={analyzerState.cursor === i}
          role="option"
          tabindex="-1"
          aria-selected={analyzerState.cursor === i}
          data-index={i}
          onclick={() => {
            analyzerState.cursor = i;
          }}
          ondblclick={() => void analyzerState.enter(entry)}
        >
          <Icon name={entry.isDir ? 'folder' : 'copy'} />
          <div class="row-main">
            <div class="row-head">
              <span class="row-title truncate">{entry.name}</span>
              <span class="row-size">{formatBytes(entry.bytes)}</span>
            </div>
            <MeterBar value={entryFraction(dir, entry.bytes)} />
          </div>
        </div>
      {/each}

      {#if dir.entries.length === 0 && !analyzerState.loading}
        <EmptyState message={t('features.cleanup.ui.folder_empty')} />
      {/if}
    {/if}

    {#if analyzerState.loading}
      <LoadingState message={t('features.cleanup.ui.scanning')} />
    {/if}
  </div>
</div>

<style>
  .analyzer-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .crumb {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5) 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .crumb-path {
    min-width: 0;
  }

  .crumb-total {
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }

  .analyzer-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-5);
    overflow-y: auto;
    flex: 1;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .row.selected {
    background: var(--bg-hover);
  }

  .row-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .row-head {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .row-title {
    color: var(--text-primary);
  }

  .row-size {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
</style>
