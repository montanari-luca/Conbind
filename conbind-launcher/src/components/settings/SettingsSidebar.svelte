<script module lang="ts">
  export interface SettingsSidebarGroup {
    id: string;
    title: string;
    items: { id: string; label: string; icon: string }[];
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import { Badge, Icon } from '..';
  import { developmentBuildIndicator } from '../../lib/developmentBuild';
  import { t } from '../../services/i18n';
  import SettingsSearch, { type SettingsSearchResult } from './SettingsSearch.svelte';

  let {
    groups,
    activeTab = $bindable(),
    query = $bindable(''),
    results,
    onpick,
  }: {
    groups: SettingsSidebarGroup[];
    activeTab: string;
    query: string;
    results: SettingsSearchResult[];
    onpick: (key: string) => void;
  } = $props();

  let nav = $state<HTMLElement | undefined>();
  let search = $state<ReturnType<typeof SettingsSearch> | undefined>();
  let pill = $state({ top: 0, height: 0, visible: false });
  // The pill lands on the first tab without travelling there from the corner.
  let settled = $state(false);

  const searching = $derived(query.trim().length > 0);
  const flatIds = $derived(groups.flatMap((group) => group.items.map((entry) => entry.id)));

  async function measure() {
    await tick();
    const el = nav?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(activeTab)}"]`);
    if (!el) {
      pill = { ...pill, visible: false };
      return;
    }
    pill = { top: el.offsetTop, height: el.offsetHeight, visible: true };
    if (!settled) requestAnimationFrame(() => (settled = true));
  }

  $effect(() => {
    void activeTab;
    void groups;
    void searching;
    measure();
  });

  /** Focus the search field from outside (Cmd/Ctrl+F, "/"). */
  export function focusSearch() {
    search?.focus();
  }

  function onNavKeydown(e: KeyboardEvent) {
    const index = flatIds.indexOf(activeTab);
    let next = -1;
    if (e.key === 'ArrowDown') next = Math.min(index + 1, flatIds.length - 1);
    else if (e.key === 'ArrowUp') next = Math.max(index - 1, 0);
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = flatIds.length - 1;
    if (next < 0) return;
    e.preventDefault();
    activeTab = flatIds[next];
    nav?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(flatIds[next])}"]`)?.focus();
  }
</script>

<aside class="settings-sidebar">
  <div class="sidebar-search">
    <SettingsSearch bind:this={search} bind:query {results} {onpick} />
  </div>

  {#if !searching}
    <!-- svelte-ignore a11y_interactive_supports_focus (roving tabindex is on the tabs) -->
    <div
      bind:this={nav}
      class="sidebar-nav custom-scrollbar"
      role="tablist"
      aria-orientation="vertical"
      aria-label={t('settings.page_title')}
      onkeydown={onNavKeydown}
    >
      <span
        class="active-pill"
        class:settled
        style:transform="translateY({pill.top}px)"
        style:height="{pill.height}px"
        style:opacity={pill.visible ? 1 : 0}
        aria-hidden="true"
      ></span>

      {#each groups as group (group.id)}
        {#if group.items.length > 0}
          <div class="nav-group-title">{group.title}</div>
          {#each group.items as tab (tab.id)}
            <button
              class="nav-item"
              class:active={activeTab === tab.id}
              data-tab-id={tab.id}
              role="tab"
              type="button"
              aria-selected={activeTab === tab.id}
              tabindex={activeTab === tab.id ? 0 : -1}
              onclick={() => (activeTab = tab.id)}
            >
              <span class="nav-icon"><Icon name={tab.icon} size={16} /></span>
              <span class="nav-label">{tab.label}</span>
            </button>
          {/each}
        {/if}
      {/each}
    </div>
  {/if}

  {#if developmentBuildIndicator}
    <div
      class="sidebar-footer"
      title={developmentBuildIndicator.title}
      aria-label={developmentBuildIndicator.title}
    >
      <Badge text={developmentBuildIndicator.text} variant="warning" mono bordered />
    </div>
  {/if}
</aside>

<style>
  .settings-sidebar {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    width: var(--settings-sidebar-width);
    min-height: 0;
    padding: var(--space-6) var(--space-5);
    gap: var(--space-5);
    border-inline-end: 1px solid var(--separator);
  }

  .sidebar-search {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .sidebar-nav {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
  }

  /* One lit pill that travels between items, rather than each item fading
     its own background in and out. */
  .active-pill {
    position: absolute;
    inset-inline: 0;
    top: 0;
    border-radius: var(--radius-md);
    background: var(--bg-selected);
    box-shadow: var(--glow-accent-soft);
    pointer-events: none;
  }

  .active-pill.settled {
    transition:
      transform var(--dur-travel) var(--ease-travel),
      height var(--dur-travel) var(--ease-travel),
      opacity var(--dur-quick) var(--ease-travel);
  }

  .active-pill::before {
    content: '';
    position: absolute;
    inset-inline-start: 0;
    top: 22%;
    bottom: 22%;
    width: var(--space-1);
    border-radius: var(--radius-full);
    background: var(--gradient-accent);
  }

  .nav-group-title {
    padding: var(--space-5) var(--space-4) var(--space-2);
    font-size: var(--font-size-2xs);
    font-weight: 600;
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
    color: var(--text-tertiary);
  }

  .nav-group-title:first-of-type {
    padding-top: 0;
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-md);
    font-weight: 500;
    text-align: start;
    cursor: pointer;
    transition:
      color var(--dur-quick) var(--ease-travel),
      background-color var(--dur-quick) var(--ease-travel);
  }

  .nav-item:hover:not(.active) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item:focus-visible {
    outline: none;
    box-shadow: var(--shadow-focus);
  }

  .nav-item.active {
    color: var(--text-primary);
  }

  .nav-icon {
    display: flex;
    color: inherit;
    transition: color var(--dur-quick) var(--ease-travel);
  }

  .nav-item.active .nav-icon {
    color: var(--accent-primary);
  }

  .nav-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sidebar-footer {
    display: flex;
    justify-content: flex-start;
    padding-inline: var(--space-4);
  }

  @media (prefers-reduced-motion: reduce) {
    .active-pill.settled {
      transition: none;
    }
  }
</style>
