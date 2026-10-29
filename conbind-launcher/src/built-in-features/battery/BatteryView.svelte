<script lang="ts">
  import { AppBar, EmptyState, LoadingState, MeterBar } from '../../components';
  import { t } from '../../services/i18n';
  import { batteryState } from './state.svelte';
  import { deriveView, splitDuration } from './batteryLogic';

  $effect(() => {
    batteryState.start();
    return () => batteryState.stop();
  });

  let snap = $derived(batteryState.snapshot);
  let view = $derived(snap ? deriveView(snap) : null);

  function duration(seconds: number): string {
    const { hours, minutes } = splitDuration(seconds);
    return hours > 0
      ? t('features.battery.ui.hours_minutes', { hours, minutes })
      : t('features.battery.ui.minutes', { minutes });
  }
</script>

<div class="battery-view">
  <AppBar title={t('features.battery.ui.title')} />

  <div class="battery-body custom-scrollbar">
    {#if snap && view?.kind === 'no-battery'}
      <EmptyState message={t('features.battery.ui.no_battery')} />
    {:else if snap && view}
      <section class="panel">
        <div class="label">
          {#if view.kind === 'time-to-full' || view.kind === 'charging'}
            {t('features.battery.ui.charging')}
          {:else}
            {t('features.battery.ui.remaining')}
          {/if}
        </div>

        {#if view.kind === 'time-left'}
          <div class="value">{t('features.battery.ui.left', { time: duration(view.seconds) })}</div>
        {:else if view.kind === 'time-to-full'}
          <div class="value">
            {t('features.battery.ui.to_full', { time: duration(view.seconds) })}
          </div>
        {:else if view.kind === 'calculating'}
          <div class="value muted">{t('features.battery.ui.calculating')}</div>
        {:else if view.kind === 'charging'}
          <div class="value muted">{t('features.battery.ui.charging_no_estimate')}</div>
        {:else if view.kind === 'charged'}
          <div class="value">{t('features.battery.ui.charged')}</div>
        {:else}
          <div class="value muted">{t('features.battery.ui.plugged')}</div>
        {/if}

        {#if snap.percent !== null}
          <MeterBar value={snap.percent / 100} />
          <div class="detail">
            {t('features.battery.ui.percent', { percent: Math.round(snap.percent) })}
          </div>
        {/if}
      </section>
    {:else if batteryState.failed}
      <EmptyState message={t('features.battery.ui.error')} />
    {:else}
      <LoadingState />
    {/if}
  </div>
</div>

<style>
  .battery-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .battery-body {
    display: flex;
    flex-direction: column;
    padding: var(--space-5);
    overflow-y: auto;
    flex: 1;
  }

  .panel {
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
</style>
