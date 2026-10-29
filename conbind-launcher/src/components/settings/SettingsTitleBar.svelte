<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { platform } from '@tauri-apps/plugin-os';
  import { onMount } from 'svelte';
  import { logService } from '../../services/log/logService';
  import { t } from '../../services/i18n';

  let { title }: { title: string } = $props();

  // macOS keeps its native traffic lights (an overlay titlebar, see
  // tauri.conf.json), so the page only reserves room for them. Everywhere else
  // the native frame is removed and the controls below stand in for it.
  let os = $state<string | null>(null);
  let maximized = $state(false);

  const isMac = $derived(os === 'macos');
  const showControls = $derived(os !== null && !isMac);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    (async () => {
      try {
        os = await platform();
      } catch (e) {
        logService.error(`Settings titlebar: failed to read platform: ${e}`);
        os = 'macos';
      }
      if (os === 'macos') return;
      const win = getCurrentWindow();
      const sync = async () => {
        try {
          maximized = await win.isMaximized();
        } catch {
          // The state icon is cosmetic; keep the last known value.
        }
      };
      await sync();
      const off = await win.onResized(sync);
      if (disposed) off();
      else unlisten = off;
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function run(action: 'minimize' | 'toggleMaximize' | 'close') {
    getCurrentWindow()
      [action]()
      .catch((e) => logService.error(`Settings titlebar: ${action} failed: ${e}`));
  }
</script>

<div class="titlebar" class:mac={isMac} data-tauri-drag-region>
  <span class="titlebar-title" data-tauri-drag-region>{title}</span>

  {#if showControls}
    <div class="window-controls">
      <button
        class="control"
        type="button"
        aria-label={t('settings.titlebar.minimize')}
        onclick={() => run('minimize')}
      >
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1 5h8" /></svg>
      </button>
      <button
        class="control"
        type="button"
        aria-label={maximized ? t('settings.titlebar.restore') : t('settings.titlebar.maximize')}
        onclick={() => run('toggleMaximize')}
      >
        {#if maximized}
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <path d="M3 3V1.5h5.5V7H7M1.5 3h5.5v5.5H1.5z" />
          </svg>
        {:else}
          <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5h7v7h-7z" /></svg>
        {/if}
      </button>
      <button
        class="control close"
        type="button"
        aria-label={t('settings.titlebar.close')}
        onclick={() => run('close')}
      >
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg>
      </button>
    </div>
  {/if}
</div>

<style>
  .titlebar {
    position: relative;
    display: flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    height: var(--settings-titlebar-height);
    border-bottom: 1px solid var(--separator);
    user-select: none;
    -webkit-user-select: none;
    cursor: default;
  }

  .titlebar-title {
    font-family: var(--font-ui);
    font-size: var(--font-size-sm);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
    color: var(--text-secondary);
  }

  .window-controls {
    position: absolute;
    inset-block: 0;
    inset-inline-end: 0;
    display: flex;
  }

  .control {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--size-2xl);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: default;
    transition:
      background-color var(--dur-instant) var(--ease-travel),
      color var(--dur-instant) var(--ease-travel);
  }

  .control svg {
    width: var(--size-xs);
    height: var(--size-xs);
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .control:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .control:focus-visible {
    outline: none;
    box-shadow: inset var(--shadow-focus);
  }

  .control.close:hover {
    background: var(--accent-danger-fill);
    color: var(--text-on-accent);
  }
</style>
