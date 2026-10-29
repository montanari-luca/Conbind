<script lang="ts">
  import { emit } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import {
    AppearanceThemeSelector,
    PaletteSelector,
    SegmentedControl,
    Select,
    SettingsCard,
    SettingsRangeSlider,
    SettingsRow,
    Toggle,
    WindowModeSelector,
    Button,
  } from '../../../components';
  import { discoverExtensions } from '../../../lib/ipc/commands';
  import { setUiScale } from '../../../lib/ipc/windowCommands';
  import { clampUiScale, DEFAULT_UI_SCALE, UI_SCALE_OPTIONS } from '../../../lib/launcher/uiScale';
  import { feedbackService } from '../../../services/feedback/feedbackService.svelte';
  import { t } from '../../../services/i18n';
  import { launcherPlacementService } from '../../../services/launcher/launcherPlacementService.svelte';
  import { logService } from '../../../services/log/logService';
  import { settingsService } from '../../../services/settings/settingsService.svelte';
  import { applyTheme, removeTheme } from '../../../services/theme/themeService';
  import type { SettingsHandler } from '../settingsHandlers.svelte';

  let {
    handler,
  }: {
    handler: SettingsHandler;
  } = $props();

  let themeExtensions = $state<
    Array<{ id: string; name: string; author?: string; version: string }>
  >([]);
  let activeThemeId = $state<string | null>(null);
  let selectedUiScale = $state(
    clampUiScale(settingsService.currentSettings.appearance.uiScale ?? DEFAULT_UI_SCALE),
  );

  onMount(async () => {
    // Separate from the theme-extension load below: Rust owns the placement,
    // so a failure there must not blank the placement controls (and vice
    // versa).
    try {
      await launcherPlacementService.load();
    } catch (e) {
      logService.error(`Failed to load launcher placement: ${e}`);
    }

    try {
      const records = await discoverExtensions();
      themeExtensions = (records ?? [])
        .filter((r: any) => r.manifest.type === 'theme' && r.enabled)
        .map((r: any) => ({
          id: r.manifest.id,
          name: r.manifest.name,
          author: r.manifest.author ?? undefined,
          version: r.manifest.version,
        }));
      activeThemeId = handler.settings?.appearance?.activeTheme ?? null;
    } catch (e) {
      logService.error(`Failed to load theme extensions: ${e}`);
      feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'warning',
        retryable: false,
        context: { message: t('settings.general.error_load_themes') },
      });
    }
  });

  const placement = launcherPlacementService;

  /** Each placement edit is one store write. Persist-then-adopt lives in the
   *  service, so a failure leaves the previous choice selected rather than a
   *  setting that isn't on disk. */
  const PLACEMENT_ERROR_KEYS: Record<string, string> = {
    display: 'settings.general.error_save_display',
    position: 'settings.general.error_save_position',
    'snap setting': 'settings.general.error_save_snap',
  };

  async function updatePlacement(change: () => Promise<void>, what: string) {
    try {
      await change();
    } catch (error) {
      logService.error(`Failed to update launcher ${what}: ${error}`);
      feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: t(PLACEMENT_ERROR_KEYS[what] ?? 'settings.general.error_save_position'),
        },
      });
    }
  }

  async function selectLaunchView(launchView: 'default' | 'compact') {
    await handler.updateLaunchView(launchView);
    await emit('conbind:launch-view-changed', { launchView });
  }

  async function selectUiScale(scale: number) {
    const clampedScale = clampUiScale(scale);
    const previousScale = selectedUiScale;
    selectedUiScale = clampedScale;

    let saved: boolean;
    try {
      saved = await settingsService.updateSettings('appearance', { uiScale: clampedScale });
    } catch {
      settingsService.currentSettings.appearance.uiScale = previousScale;
      selectedUiScale = previousScale;
      reportUiScaleError();
      return;
    }

    if (!saved) {
      settingsService.currentSettings.appearance.uiScale = previousScale;
      selectedUiScale = previousScale;
      reportUiScaleError();
      return;
    }

    try {
      await setUiScale(clampedScale);
    } catch {
      reportUiScaleError();
    }
  }

  function reportUiScaleError() {
    logService.error('Failed to update interface size');
    feedbackService.report({
      source: 'frontend',
      kind: 'manual',
      severity: 'error',
      retryable: false,
      context: { message: t('settings.general.error_save_interface_size') },
    });
  }

  async function selectTheme(themeId: string | null) {
    try {
      if (themeId) {
        await applyTheme(themeId);
      } else {
        removeTheme();
      }
      activeThemeId = themeId;
      await settingsService.updateSettings('appearance', { activeTheme: themeId });
      await emit('conbind:theme-changed', { themeId });
    } catch (error) {
      logService.error(`Failed to apply theme ${themeId}: ${error}`);
      feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: {
          message: themeId
            ? `Could not apply theme "${themeId}"`
            : t('settings.general.error_remove_active_theme'),
        },
      });
    }
  }
</script>

<div class="section-header">{t('settings.general.section_appearance')}</div>
<SettingsCard>
  <div id="general-appearance">
    <SettingsRow
      label={t('settings.appearance.theme')}
      description={t('settings.general.theme_description')}
    >
      <AppearanceThemeSelector
        value={handler.selectedTheme as 'light' | 'dark' | 'system'}
        onchange={(v) => handler.updateThemeSetting(v)}
        wellBackground="secondary"
      />
    </SettingsRow>
    <SettingsRow
      label={t('settings.appearance.palette')}
      description={t('settings.appearance.palette_description')}
    >
      <PaletteSelector
        value={handler.selectedPalette}
        onchange={(v) => handler.updatePaletteSetting(v)}
      />
    </SettingsRow>
    <SettingsRow
      label={t('settings.appearance.window_mode')}
      description={t('settings.appearance.window_mode_description')}
    >
      <WindowModeSelector
        value={handler.selectedLaunchView}
        onchange={selectLaunchView}
        wellBackground="primary"
      />
    </SettingsRow>
    <SettingsRow
      label={t('settings.appearance.interface_size_label')}
      description={t('settings.appearance.interface_size_hint')}
    >
      <Select
        value={String(selectedUiScale)}
        options={UI_SCALE_OPTIONS.map((option) => ({
          value: String(option.value),
          label: t(option.labelKey),
        }))}
        onchange={(value) => selectUiScale(Number(value))}
      />
    </SettingsRow>
    {#if handler.selectedLaunchView === 'compact'}
      <SettingsRow
        label={t('settings.appearance.hide_show_more')}
        description={t('settings.appearance.hide_show_more_description')}
      >
        <Toggle
          checked={!!handler.settings.appearance.hideCompactShowMore}
          onchange={() =>
            settingsService.updateSettings('appearance', {
              hideCompactShowMore: !handler.settings.appearance.hideCompactShowMore,
            })}
        />
      </SettingsRow>
    {/if}
  </div>
</SettingsCard>

<div class="section-header">{t('settings.general.section_placement')}</div>
<SettingsCard>
  <div id="general-placement">
    <SettingsRow
      label={t('settings.general.display')}
      description={t('settings.general.display_description')}
    >
      <SegmentedControl
        options={[
          { value: 'cursor', label: t('settings.general.display_cursor') },
          { value: 'primary', label: t('settings.general.display_primary') },
        ]}
        value={placement.placement.monitor}
        onchange={(v) =>
          updatePlacement(() => placement.setMonitor(v as 'cursor' | 'primary'), 'display')}
      />
    </SettingsRow>
    <SettingsRow
      label={t('settings.general.vertical_position')}
      description={t('settings.general.vertical_position_description')}
    >
      <SegmentedControl
        options={[
          { value: 'top', label: t('settings.general.vertical_top') },
          { value: 'center', label: t('settings.general.vertical_center') },
          { value: 'custom', label: t('settings.general.vertical_custom') },
        ]}
        value={placement.vertical ?? ''}
        onchange={(v) =>
          updatePlacement(
            () => placement.setVertical(v as 'top' | 'center' | 'custom'),
            'position',
          )}
      />
    </SettingsRow>
    {#if placement.vertical === 'custom'}
      <SettingsRow label={t('settings.general.distance_from_top')}>
        <SettingsRangeSlider
          min={0}
          max={100}
          value={placement.biasPercent}
          suffix="%"
          onchange={(v) => updatePlacement(() => placement.setBias(v), 'position')}
        />
      </SettingsRow>
    {/if}
    <SettingsRow
      label={t('settings.general.snap_dragging')}
      description={t('settings.general.snap_dragging_description')}
    >
      <Toggle
        checked={placement.placement.snapEnabled}
        onchange={() =>
          updatePlacement(
            () => placement.setSnapEnabled(!placement.placement.snapEnabled),
            'snap setting',
          )}
      />
    </SettingsRow>
    {#if placement.isDragged}
      <SettingsRow
        label={t('settings.general.custom_position')}
        description={t('settings.general.custom_position_description')}
      >
        <Button
          class="btn-secondary"
          onclick={() => updatePlacement(() => placement.reset(), 'position')}
        >
          {t('settings.general.reset')}
        </Button>
      </SettingsRow>
    {/if}
  </div>
</SettingsCard>

{#if themeExtensions.length > 0}
  <div class="section-header">{t('settings.general.custom_themes')}</div>
  <SettingsCard>
    <div class="themes-list">
      <label class="theme-item" class:theme-active={activeThemeId === null}>
        <input
          type="radio"
          name="custom-theme"
          checked={activeThemeId === null}
          onchange={() => selectTheme(null)}
          class="sr-only"
        />
        <div class="theme-item-body">
          <div class="theme-item-name">{t('settings.general.default_theme')}</div>
          <div class="theme-item-meta">{t('settings.general.default_theme_meta')}</div>
        </div>
      </label>

      {#each themeExtensions as theme}
        <label class="theme-item" class:theme-active={activeThemeId === theme.id}>
          <input
            type="radio"
            name="custom-theme"
            checked={activeThemeId === theme.id}
            onchange={() => selectTheme(theme.id)}
            class="sr-only"
          />
          <div class="theme-item-body">
            <div class="theme-item-name">{theme.name}</div>
            <div class="theme-item-meta">
              {#if theme.author}{theme.author} &middot;
              {/if}v{theme.version}
            </div>
          </div>
        </label>
      {/each}
    </div>
  </SettingsCard>
{/if}

<style>
  .themes-list {
    display: flex;
    flex-direction: column;
  }

  .theme-item {
    position: relative;
    display: flex;
    align-items: center;
    padding: var(--space-5) var(--space-6);
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .theme-item:not(:last-child)::after {
    content: '';
    position: absolute;
    left: var(--space-6);
    right: 0;
    bottom: 0;
    height: 1px;
    background: var(--border-color);
  }

  .theme-item:hover {
    background: var(--bg-hover);
  }

  .theme-item.theme-active .theme-item-name {
    color: var(--accent-primary);
  }

  .theme-item-body {
    flex: 1;
  }

  .theme-item-name {
    font-weight: 500;
    font-size: var(--font-size-md);
    color: var(--text-primary);
    font-family: var(--font-ui);
  }

  .theme-item-meta {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    font-family: var(--font-ui);
    margin-top: var(--space-1);
  }
</style>
