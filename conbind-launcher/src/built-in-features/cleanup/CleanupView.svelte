<script lang="ts">
  import { AppBar, Button, Checkbox, EmptyState, LoadingState, MeterBar } from '../../components';
  import { formatBytes } from '../../services/action/actionService.svelte';
  import { t } from '../../services/i18n';
  import { scrollSelectedIntoView } from '../../lib/listScroll';
  import FullDiskAccessBanner from './FullDiskAccessBanner.svelte';
  import { cleanupState } from './state.svelte';
  import { selectionState } from './cleanupLogic';

  let listEl = $state<HTMLDivElement | undefined>();

  let title = $derived(
    cleanupState.group === 'developer'
      ? t('features.cleanup.ui.title_developer')
      : cleanupState.group === 'leftovers'
        ? t('features.cleanup.ui.title_leftovers')
        : t('features.cleanup.ui.title'),
  );
  let usedFraction = $derived(
    cleanupState.disk && cleanupState.disk.totalBytes > 0
      ? 1 - cleanupState.disk.freeBytes / cleanupState.disk.totalBytes
      : 0,
  );
  let nothingFound = $derived(!cleanupState.scanning && cleanupState.rows.length === 0);

  $effect(() => {
    const index = cleanupState.cursor;
    if (listEl) scrollSelectedIntoView(listEl, index);
  });
</script>

<div class="cleanup-view">
  <AppBar {title} />

  {#if cleanupState.group === 'leftovers'}
    <FullDiskAccessBanner />
    <div class="note">{t('features.cleanup.ui.leftovers_note')}</div>
  {/if}

  {#if cleanupState.disk}
    <div class="disk">
      <div class="disk-text">
        {t('features.cleanup.ui.disk_free', {
          free: formatBytes(cleanupState.disk.freeBytes),
          total: formatBytes(cleanupState.disk.totalBytes),
        })}
      </div>
      <MeterBar value={usedFraction} variant="neutral" />
    </div>
  {/if}

  <div class="cleanup-body custom-scrollbar" bind:this={listEl} role="listbox">
    {#each cleanupState.rows as row, i (row.kind === 'category' ? row.category.category : row.item.id)}
      {#if row.kind === 'category'}
        {@const state = selectionState(row.category, cleanupState.selected)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="row category"
          class:selected={cleanupState.cursor === i}
          role="option"
          tabindex="-1"
          aria-selected={cleanupState.cursor === i}
          data-index={i}
          onclick={() => {
            cleanupState.cursor = i;
            cleanupState.toggleExpanded(row.category.category);
          }}
        >
          <Checkbox
            checked={state === 'all'}
            onchange={() => cleanupState.toggleCategory(row.category)}
          />
          <div class="row-main">
            <div class="row-title">
              {t(`features.cleanup.category.${row.category.category}`)}
            </div>
            <div class="row-sub">
              {t('features.cleanup.ui.items', { count: row.category.items.length })}
              {#if state === 'some'}· {t('features.cleanup.ui.partly_selected')}{/if}
            </div>
          </div>
          <div class="row-size">{formatBytes(row.category.bytes)}</div>
        </div>
      {:else}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="row item"
          class:selected={cleanupState.cursor === i}
          role="option"
          tabindex="-1"
          aria-selected={cleanupState.cursor === i}
          data-index={i}
          onclick={() => {
            cleanupState.cursor = i;
            cleanupState.toggleItem(row.item.id);
          }}
        >
          <Checkbox
            checked={cleanupState.selected.has(row.item.id)}
            onchange={() => cleanupState.toggleItem(row.item.id)}
          />
          <div class="row-main">
            <div class="row-title truncate">{row.item.label}</div>
            <div class="row-sub truncate">{row.item.path}</div>
          </div>
          <div class="row-size">{formatBytes(row.item.bytes)}</div>
        </div>
      {/if}
    {/each}

    {#if cleanupState.scanning}
      <LoadingState message={t('features.cleanup.ui.scanning')} />
    {:else if nothingFound}
      <EmptyState
        message={t('features.cleanup.ui.nothing')}
        description={t('features.cleanup.ui.nothing_description')}
      />
    {/if}
  </div>

  <div class="cleanup-footer">
    <div class="footer-text">
      {t('features.cleanup.ui.selected', {
        size: formatBytes(cleanupState.totals.bytes),
        count: cleanupState.totals.count,
      })}
    </div>
    <Button
      class="btn-primary"
      disabled={cleanupState.totals.count === 0 || cleanupState.cleaning || cleanupState.scanning}
      onclick={() => void cleanupState.requestClean()}
    >
      {t('features.cleanup.ui.trash')}
    </Button>
  </div>
</div>

<style>
  .cleanup-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .disk {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-5) 0;
  }

  .note {
    padding: var(--space-3) var(--space-5) 0;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .disk-text {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .cleanup-body {
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

  .row.item {
    margin-left: var(--space-6);
  }

  .row-main {
    flex: 1;
    min-width: 0;
  }

  .row-title {
    color: var(--text-primary);
  }

  .row-sub {
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
  }

  .row-size {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .cleanup-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--border-color);
  }

  .footer-text {
    color: var(--text-secondary);
  }
</style>
