<script lang="ts">
  import { onMount } from 'svelte';
  import { normalizeShortcut } from '../../../built-in-features/shortcuts/shortcutFormatter';
  import { shortcutService } from '../../../built-in-features/shortcuts/shortcutService';
  import {
    Button,
    Checkbox,
    SettingsCard,
    SettingsRow,
    ShortcutRecorder,
  } from '../../../components';
  import {
    getShortcutBackendInfo,
    onboardingCommands,
    type ShortcutBackendInfo,
  } from '../../../lib/ipc/commands';
  import { feedbackService } from '../../../services/feedback/feedbackService.svelte';
  import { t } from '../../../services/i18n';
  import { logService } from '../../../services/log/logService';
  import { getWaylandBindingGuide } from '../../onboarding/steps/summonSearchSetup';
  import type { SettingsHandler } from '../settingsHandlers.svelte';

  let {
    handler,
  }: {
    handler: SettingsHandler;
  } = $props();

  let shortcutBackendInfo = $state<ShortcutBackendInfo | null>(null);
  let waylandCopied = $state(false);
  const waylandGuide = $derived(getWaylandBindingGuide(shortcutBackendInfo));

  async function copyWaylandCommand() {
    if (!waylandGuide) return;
    try {
      await navigator.clipboard.writeText(waylandGuide.command);
      waylandCopied = true;
    } catch {
      // Clipboard write may fail in restricted environments; ignore.
    }
  }

  onMount(async () => {
    try {
      shortcutBackendInfo = await getShortcutBackendInfo();
    } catch {
      shortcutBackendInfo = { isWayland: false, compositor: null, isOmarchy: false };
    }
  });

  async function conflictChecker(shortcut: string): Promise<{ name: string } | null> {
    const conflict = await shortcutService.isConflict(normalizeShortcut(shortcut), 'launcher');
    if (conflict) return { name: conflict.itemName };
    return null;
  }

  async function handleSave(detail: { modifier: string; key: string }): Promise<string | true> {
    handler.selectedModifier = detail.modifier;
    handler.selectedKey = detail.key;
    handler.isSaving = true;
    handler.saveMessage = '';
    handler.saveError = false;

    try {
      const { updateShortcut } = await import('../../../utils/shortcutManager');
      const success = await updateShortcut(detail.modifier, detail.key);
      handler.isSaving = false;
      if (success) return true;
      return 'Cannot save, shortcut may be reserved by the OS or another app';
    } catch (e) {
      handler.isSaving = false;
      return 'Cannot save, shortcut may be reserved by the OS or another app';
    }
  }

  async function rerunOnboarding() {
    try {
      await onboardingCommands.reset();
    } catch (e) {
      logService.error(`Failed to re-run onboarding: ${e}`);
      feedbackService.report({
        source: 'frontend',
        kind: 'manual',
        severity: 'error',
        retryable: false,
        context: { message: t('settings.general.error_rerun_onboarding') },
      });
    }
  }
</script>

<div class="section-header">{t('settings.general.section_startup')}</div>
<SettingsCard>
  <div id="general-startup">
    <SettingsRow
      label={t('settings.general.autostart')}
      description={t('settings.general.autostart_description')}
    >
      <Checkbox
        checked={handler.settings.general.startAtLogin}
        onchange={() => handler.handleAutostartToggle()}
      />
    </SettingsRow>
    <SettingsRow
      label={t('settings.general.hotkey')}
      description={t('settings.general.hotkey_description')}
    >
      <ShortcutRecorder
        bind:modifier={handler.selectedModifier}
        bind:key={handler.selectedKey}
        placeholder={t('settings.general.hotkey_placeholder')}
        disabled={handler.isSaving}
        onsave={handleSave}
        {conflictChecker}
      />
    </SettingsRow>
    {#if waylandGuide}
      <div class="wayland-hotkey-hint">
        <p class="wayland-hotkey-text">
          {t('settings.general.wayland_hotkey_hint', { compositor: waylandGuide.compositorLabel })}
        </p>
        <div class="wayland-hotkey-row">
          <pre class="wayland-hotkey-code"><code>{waylandGuide.command}</code></pre>
          <Button variant="secondary" size="sm" onclick={copyWaylandCommand}>
            {waylandCopied
              ? t('settings.general.wayland_copied')
              : t('settings.general.wayland_copy_cmd')}
          </Button>
        </div>
      </div>
    {/if}
    <SettingsRow
      label={t('settings.general.show_tray_icon')}
      description={handler.settings.general.showTrayIcon
        ? t('settings.general.show_tray_icon_description')
        : t('settings.general.show_tray_icon_tip')}
    >
      <Checkbox
        checked={handler.settings.general.showTrayIcon}
        onchange={() => handler.handleTrayIconToggle()}
      />
    </SettingsRow>
    <SettingsRow
      label={t('settings.general.show_dock_icon')}
      description={t('settings.general.show_dock_icon_description')}
    >
      <Checkbox
        checked={handler.settings.general.showDockIcon}
        onchange={() => handler.handleDockIconToggle()}
      />
    </SettingsRow>
  </div>
</SettingsCard>

<SettingsCard>
  <div id="general-onboarding">
    <SettingsRow
      label={t('settings.general.onboarding')}
      description={t('settings.general.onboarding_description')}
    >
      <Button class="btn-secondary" onclick={rerunOnboarding}
        >{t('settings.general.rerun_onboarding')}</Button
      >
    </SettingsRow>
  </div>
</SettingsCard>

<style>
  .wayland-hotkey-hint {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-6);
    background: var(--bg-tertiary);
    border-top: 1px solid var(--separator);
  }

  .wayland-hotkey-text {
    margin: 0;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .wayland-hotkey-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .wayland-hotkey-code {
    flex: 1;
    margin: 0;
    padding: var(--space-2);
    background: var(--bg-secondary);
    border: 1px solid var(--separator);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--text-primary);
    overflow-x: auto;
  }
</style>
