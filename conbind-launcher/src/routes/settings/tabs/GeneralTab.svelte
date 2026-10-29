<script lang="ts">
  import { normalizeShortcut } from '../../../built-in-features/shortcuts/shortcutFormatter';
  import { shortcutService } from '../../../built-in-features/shortcuts/shortcutService';
  import {
    Button,
    Checkbox,
    SettingsCard,
    SettingsRow,
    ShortcutRecorder,
  } from '../../../components';
  import { onboardingCommands } from '../../../lib/ipc/commands';
  import { feedbackService } from '../../../services/feedback/feedbackService.svelte';
  import { t } from '../../../services/i18n';
  import { logService } from '../../../services/log/logService';
  import type { SettingsHandler } from '../settingsHandlers.svelte';

  let {
    handler,
  }: {
    handler: SettingsHandler;
  } = $props();

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
