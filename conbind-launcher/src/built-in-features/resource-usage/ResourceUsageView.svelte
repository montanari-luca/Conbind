<script lang="ts">
  import { AppBar, EmptyState, LoadingState, MeterBar } from '../../components';
  import { t } from '../../services/i18n';
  import { resourceUsageState } from './state.svelte';
  import { formatPercent, formatRate, formatSize, fraction, vramFraction } from './resourceLogic';

  $effect(() => {
    resourceUsageState.start();
    return () => resourceUsageState.stop();
  });

  let snap = $derived(resourceUsageState.snapshot);
  let gpus = $derived(snap?.gpus ?? []);
  let multiGpu = $derived(gpus.length > 1);
</script>

<div class="usage-view">
  <AppBar title={t('features.resource_usage.ui.title')} />

  <div class="usage-body custom-scrollbar">
    {#if snap}
      <div class="grid">
        <section class="panel">
          <div class="label">{t('features.resource_usage.ui.cpu')}</div>
          <div class="value">{formatPercent(snap.cpu.percent)}</div>
          <MeterBar value={snap.cpu.percent / 100} />
          <div class="detail">
            {t(
              snap.cpu.cores === 1
                ? 'features.resource_usage.ui.cores_one'
                : 'features.resource_usage.ui.cores_other',
              { count: snap.cpu.cores },
            )}
          </div>
        </section>

        <section class="panel">
          <div class="label">{t('features.resource_usage.ui.ram')}</div>
          <div class="value">
            {formatPercent(fraction(snap.memory.used, snap.memory.total) * 100)}
          </div>
          <MeterBar value={fraction(snap.memory.used, snap.memory.total)} />
          <div class="detail">
            {t('features.resource_usage.ui.used_of', {
              used: formatSize(snap.memory.used),
              total: formatSize(snap.memory.total),
            })}
          </div>
          {#if snap.memory.swapTotal > 0}
            <div class="detail">
              {t('features.resource_usage.ui.swap', {
                used: formatSize(snap.memory.swapUsed),
                total: formatSize(snap.memory.swapTotal),
              })}
            </div>
          {/if}
        </section>

        <section class="panel">
          <div class="label">{t('features.resource_usage.ui.gpu')}</div>
          {#each gpus as gpu, i (i)}
            <div class="gpu-row">
              {#if multiGpu}<div class="detail name">{gpu.name}</div>{/if}
              {#if gpu.usagePercent !== null}
                <div class="value">{formatPercent(gpu.usagePercent)}</div>
                <MeterBar value={gpu.usagePercent / 100} />
              {:else}
                <div class="value muted">{t('features.resource_usage.ui.not_available')}</div>
              {/if}
              {#if !multiGpu}<div class="detail name">{gpu.name}</div>{/if}
            </div>
          {:else}
            <div class="value muted">{t('features.resource_usage.ui.not_available')}</div>
          {/each}
        </section>

        <section class="panel">
          <div class="label">{t('features.resource_usage.ui.vram')}</div>
          {#each gpus as gpu, i (i)}
            {@const vram = vramFraction(gpu)}
            <div class="gpu-row">
              {#if multiGpu}<div class="detail name">{gpu.name}</div>{/if}
              {#if vram !== null && gpu.vramUsed !== null && gpu.vramTotal !== null}
                <div class="value">{formatPercent(vram * 100)}</div>
                <MeterBar value={vram} />
                <div class="detail">
                  {t('features.resource_usage.ui.used_of', {
                    used: formatSize(gpu.vramUsed),
                    total: formatSize(gpu.vramTotal),
                  })}
                </div>
              {:else}
                <div class="value muted">{t('features.resource_usage.ui.not_available')}</div>
              {/if}
            </div>
          {:else}
            <div class="value muted">{t('features.resource_usage.ui.not_available')}</div>
          {/each}
        </section>

        <section class="panel">
          <div class="label">{t('features.resource_usage.ui.network')}</div>
          <div class="net-row">
            <span class="detail">{t('features.resource_usage.ui.download')}</span>
            <span class="net-value">{formatRate(snap.network.rxPerSec)}</span>
          </div>
          <div class="net-row">
            <span class="detail">{t('features.resource_usage.ui.upload')}</span>
            <span class="net-value">{formatRate(snap.network.txPerSec)}</span>
          </div>
          <div class="detail">
            {t('features.resource_usage.ui.session_totals', {
              rx: formatSize(snap.network.totalRx),
              tx: formatSize(snap.network.totalTx),
            })}
          </div>
        </section>

        <section class="panel">
          <div class="label">{t('features.resource_usage.ui.disk')}</div>
          {#each snap.disks as disk (disk.mount)}
            <div class="disk-row">
              <div class="net-row">
                <span class="detail name truncate">{disk.name || disk.mount}</span>
                <span class="detail">{formatPercent(fraction(disk.used, disk.total) * 100)}</span>
              </div>
              <MeterBar value={fraction(disk.used, disk.total)} variant="neutral" />
              <div class="detail">
                {t('features.resource_usage.ui.used_of', {
                  used: formatSize(disk.used),
                  total: formatSize(disk.total),
                })}
              </div>
            </div>
          {:else}
            <div class="value muted">{t('features.resource_usage.ui.not_available')}</div>
          {/each}
        </section>
      </div>
    {:else if resourceUsageState.failed}
      <EmptyState message={t('features.resource_usage.ui.error')} />
    {:else}
      <LoadingState />
    {/if}
  </div>
</div>

<style>
  .usage-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .usage-body {
    display: flex;
    flex-direction: column;
    padding: var(--space-5);
    overflow-y: auto;
    flex: 1;
  }

  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .panel {
    flex: 1 1 40%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
  }

  .label {
    font-size: var(--font-size-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-tertiary);
  }

  .value {
    font-size: var(--font-size-display);
    font-weight: 600;
    line-height: 1;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .value.muted {
    font-size: var(--font-size-sm);
    font-weight: 400;
    letter-spacing: 0;
    color: var(--text-tertiary);
  }

  .detail {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .detail.name {
    min-width: 0;
  }

  .gpu-row,
  .disk-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .net-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-3);
  }

  .net-value {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
</style>
