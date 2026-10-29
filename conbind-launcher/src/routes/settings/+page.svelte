<!-- src/routes/settings/+page.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';
  import {
    LoadingState,
    SettingsPageHeader,
    SettingsSidebar,
    SettingsTitleBar,
  } from '../../components';
  import type { SettingsSidebarGroup } from '../../components/settings/SettingsSidebar.svelte';
  import { SettingsHandler } from './settingsHandlers.svelte';
  import { findNavItem, visibleNavGroups, type SettingsTabId } from './settingsNav';
  import { searchSettings, type SettingsSearchHit } from './settingsIndex';
  import GeneralTab from './tabs/GeneralTab.svelte';
  import AppearanceTab from './tabs/AppearanceTab.svelte';
  import AiTab from './tabs/AiTab.svelte';
  import ApplicationsTab from './tabs/ApplicationsTab.svelte';
  import FileSearchTab from './tabs/FileSearchTab.svelte';
  import ScriptsTab from './tabs/ScriptsTab.svelte';
  import ExtensionsTab from './tabs/ExtensionsTab.svelte';
  import AboutTab from './tabs/AboutTab.svelte';
  import BackupTab from './tabs/BackupTab.svelte';
  import AdvancedTab from './tabs/AdvancedTab.svelte';
  import DeveloperTab from './tabs/DeveloperTab.svelte';
  import PrivacyTab from './tabs/PrivacyTab.svelte';
  import BrowsersTab from './tabs/BrowsersTab.svelte';
  import { registerProfileProviders } from '../../services/appInitializer';
  import { initProviders } from '../../services/ai/initProviders';
  import { shortcutStore } from '../../built-in-features/shortcuts/shortcutStore.svelte';
  import { snippetStore } from '../../built-in-features/snippets/snippetStore.svelte';
  import { noteStore } from '../../built-in-features/notes/noteStore.svelte';
  import { portalStore } from '../../built-in-features/portals/portalStore.svelte';
  import { initValidKeys } from '../../built-in-features/shortcuts/shortcutFormatter';
  import { listen } from '@tauri-apps/api/event';
  import { i18nService, t } from '../../services/i18n';
  import {
    initAppUpdateStore,
    destroyAppUpdateStore,
  } from '../../services/update/appUpdateStore.svelte';

  import '../../resources/styles/style.css';

  // Settings is a separate webview window — register AI providers locally
  // before AiTab reads the provider registry.
  initProviders();

  const handler = new SettingsHandler();

  const navGroups = $derived<SettingsSidebarGroup[]>(
    visibleNavGroups(!!handler.settings.developer?.enabled).map((group) => ({
      id: group.id,
      title: t(group.titleKey),
      items: group.items.map((entry) => ({
        id: entry.id,
        label: t(entry.labelKey),
        icon: entry.icon,
      })),
    })),
  );

  const activeNav = $derived(findNavItem(handler.activeTab));
  const fullBleed = $derived(handler.activeTab === 'extensions');

  // Search. Hits are resolved against the translated text, so results follow
  // the UI language.
  let query = $state('');
  let sidebar = $state<ReturnType<typeof SettingsSidebar> | undefined>();
  const visibleTabs = $derived(
    new Set<SettingsTabId>(navGroups.flatMap((g) => g.items.map((i) => i.id as SettingsTabId))),
  );
  const hits = $derived<SettingsSearchHit[]>(
    query.trim() ? searchSettings(query, t, visibleTabs) : [],
  );
  const results = $derived(
    hits.map((hit, i) => ({
      key: String(i),
      label: hit.label,
      description: hit.description,
      pageLabel: hit.pageLabel,
    })),
  );

  const reducedMotion =
    typeof window !== 'undefined' &&
    !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

  /** Scroll a row into view and flash it. The page it lives on may only mount
   *  a frame or two after the tab switch, so look for it briefly. */
  function revealRow(label: string, attemptsLeft = 30) {
    const row = Array.from(document.querySelectorAll<HTMLElement>('.settings-row-label'))
      .find((el) => el.textContent?.trim() === label)
      ?.closest<HTMLElement>('.settings-row');
    if (!row) {
      if (attemptsLeft > 0) requestAnimationFrame(() => revealRow(label, attemptsLeft - 1));
      return;
    }
    row.scrollIntoView({ block: 'center', behavior: reducedMotion ? 'auto' : 'smooth' });
    row.classList.remove('pulse');
    void row.offsetWidth;
    row.classList.add('pulse');
    row.addEventListener('animationend', () => row.classList.remove('pulse'), { once: true });
  }

  function pickResult(key: string) {
    const hit = hits[Number(key)];
    if (!hit) return;
    query = '';
    handler.activeTab = hit.tab;
    if (hit.rowLabel) revealRow(hit.rowLabel);
  }

  function onWindowKeydown(e: KeyboardEvent) {
    const typing =
      e.target instanceof HTMLElement &&
      (e.target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target.tagName));
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      sidebar?.focusSearch();
    } else if (e.key === '/' && !typing && !e.metaKey && !e.ctrlKey && !e.altKey) {
      e.preventDefault();
      sidebar?.focusSearch();
    }
  }

  let unlistenNavTab: (() => void) | undefined;

  onMount(async () => {
    void i18nService.init();
    handler.init();
    await Promise.all([
      shortcutStore.init(),
      snippetStore.init(),
      noteStore.init(),
      portalStore.init(),
    ]);
    await initValidKeys();
    registerProfileProviders();
    void initAppUpdateStore();
    unlistenNavTab = await listen<{ tab: string; extensionId?: string | null }>(
      'conbind:navigate-settings-tab',
      (e) => {
        handler.activeTab = e.payload.tab;
        if (e.payload.extensionId) {
          handler.pendingExtensionSelection = e.payload.extensionId;
        }
      },
    );
  });

  onDestroy(() => {
    handler.destroy();
    unlistenNavTab?.();
    destroyAppUpdateStore();
  });
</script>

<svelte:head>
  <title>{t('settings.page_title')}</title>
</svelte:head>

<svelte:window onkeydown={onWindowKeydown} />

<div class="settings-page">
  <SettingsTitleBar title={t('settings.page_title')} />

  {#if handler.isLoading}
    <div class="settings-loading">
      <LoadingState message={t('settings.loading')} />
    </div>
  {:else}
    <div class="settings-body">
      <SettingsSidebar
        bind:this={sidebar}
        groups={navGroups}
        bind:activeTab={handler.activeTab}
        bind:query
        {results}
        onpick={pickResult}
      />

      <div class="settings-main" class:full-bleed={fullBleed}>
        {#if handler.initError}
          <div class="init-error">⚠️ {handler.initError}</div>
        {/if}

        {#key handler.activeTab}
          <div
            class="settings-scroll custom-scrollbar"
            class:full-bleed={fullBleed}
            in:fly={{ y: reducedMotion ? 0 : 8, duration: reducedMotion ? 0 : 220 }}
          >
            <div
              class="settings-content-inner"
              class:full-bleed-inner={fullBleed}
              class:wide-inner={handler.activeTab === 'applications'}
            >
              <div class="page-head">
                <SettingsPageHeader
                  title={activeNav ? t(activeNav.labelKey) : ''}
                  subtitle={activeNav ? t(activeNav.subtitleKey) : ''}
                />
              </div>

              {#if handler.activeTab === 'general'}
                <GeneralTab {handler} />
              {:else if handler.activeTab === 'appearance'}
                <AppearanceTab {handler} />
              {:else if handler.activeTab === 'ai'}
                <AiTab {handler} />
              {:else if handler.activeTab === 'extensions'}
                <ExtensionsTab {handler} />
              {:else if handler.activeTab === 'browsers'}
                <BrowsersTab />
              {:else if handler.activeTab === 'applications'}
                <ApplicationsTab />
              {:else if handler.activeTab === 'file-search'}
                <FileSearchTab />
              {:else if handler.activeTab === 'scripts'}
                <ScriptsTab />
              {:else if handler.activeTab === 'backup'}
                <BackupTab {handler} />
              {:else if handler.activeTab === 'privacy'}
                <PrivacyTab />
              {:else if handler.activeTab === 'advanced'}
                <AdvancedTab {handler} />
              {:else if handler.activeTab === 'developer'}
                <DeveloperTab {handler} />
              {:else if handler.activeTab === 'about'}
                <AboutTab {handler} />
              {/if}
            </div>
          </div>
        {/key}
      </div>
    </div>
  {/if}
</div>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    height: 100vh;
    /* No fill of its own: the window is transparent and the root element's
       tint sits over the native vibrancy, exactly as in the launcher. */
    background: transparent;
  }

  /* Windows: the root is transparent and DWM paints Acrylic, so the theme
     tint lives on the page, as it does for the launcher. */
  :global(html[data-platform='windows']) .settings-page,
  :global(html[data-platform='win32']) .settings-page {
    background: var(--win-acrylic-tint);
  }

  .settings-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .settings-loading {
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: center;
  }

  .settings-main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .init-error {
    flex-shrink: 0;
    padding: var(--space-3);
    text-align: center;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--accent-warning) 15%, var(--bg-primary));
  }

  .settings-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-8) var(--space-9);
  }

  /* The extensions manager is a master/detail pane that owns its own
     scrolling, so it gets the whole column under a padded header. */
  .settings-scroll.full-bleed {
    padding: 0;
    overflow: hidden;
  }

  .settings-content-inner {
    max-width: 860px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  /* Section labels in the settings window sit flush with the cards and carry a
     small accent mark. Scoped here because .section-header is shared with the
     launcher's feature views. */
  .settings-content-inner :global(.section-header) {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-inline: var(--space-1);
    margin-bottom: calc(var(--space-3) * -1);
  }

  .settings-content-inner :global(.section-header)::before {
    content: '';
    width: var(--space-1);
    height: var(--font-size-sm);
    border-radius: var(--radius-full);
    background: var(--gradient-accent);
  }

  .settings-content-inner.wide-inner {
    max-width: 920px;
  }

  .settings-content-inner.full-bleed-inner {
    max-width: none;
    margin: 0;
    height: 100%;
    gap: 0;
  }

  .full-bleed-inner .page-head {
    flex-shrink: 0;
    padding: var(--space-8) var(--space-9) var(--space-6);
  }

  .full-bleed-inner > :global(:not(.page-head)) {
    flex: 1;
    min-height: 0;
  }
</style>
