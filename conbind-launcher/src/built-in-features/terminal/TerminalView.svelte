<script lang="ts">
  import { tick } from 'svelte';
  import { AppBar } from '../../components';
  import { t } from '../../services/i18n';
  import { terminalState } from './terminalState.svelte';
  import { displayCwd } from './terminalLogic';

  let input = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);
  let scrollEl = $state<HTMLDivElement | null>(null);
  // Position in the history while pressing arrow keys; -1 is the unsent draft.
  let historyIndex = -1;
  let draft = '';

  let running = $derived(terminalState.running);
  let prompt = $derived(displayCwd(terminalState.cwd, terminalState.home));

  $effect(() => {
    void terminalState.init().then(() => {
      const pending = terminalState.takePending();
      if (pending) void terminalState.run(pending);
    });
    inputEl?.focus();
  });

  // Follow the output as it streams in.
  $effect(() => {
    void terminalState.blocks.length;
    void terminalState.blocks[terminalState.blocks.length - 1]?.lines.length;
    void tick().then(() => {
      if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
    });
  });

  function submit() {
    const command = input.trim().replace(/^>\s*/, '');
    if (!command || running) return;
    input = '';
    historyIndex = -1;
    void terminalState.run(command);
  }

  function browseHistory(direction: -1 | 1) {
    const history = terminalState.history;
    if (history.length === 0) return;
    if (historyIndex === -1) {
      if (direction === 1) return;
      draft = input;
      historyIndex = history.length - 1;
    } else {
      historyIndex += -direction;
      if (historyIndex >= history.length) {
        historyIndex = -1;
        input = draft;
        return;
      }
      historyIndex = Math.max(historyIndex, 0);
    }
    input = history[historyIndex];
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.isComposing) {
      e.preventDefault();
      submit();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      browseHistory(-1);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      browseHistory(1);
    } else if (e.ctrlKey && e.key.toLowerCase() === 'c' && running) {
      e.preventDefault();
      void terminalState.cancel();
    }
  }
</script>

<div class="terminal-view">
  <AppBar title={t('features.terminal.ui.title')} />

  <div class="scrollback custom-scrollbar" bind:this={scrollEl}>
    {#if terminalState.blocks.length === 0}
      <div class="hint">{t('features.terminal.ui.hint')}</div>
    {/if}
    {#each terminalState.blocks as block (block.id)}
      <section class="block">
        <div class="command-line">
          <span class="cwd">{displayCwd(block.cwd, terminalState.home)}</span>
          <span class="command">{block.command}</span>
        </div>
        {#if block.lines.length > 0}
          <pre class="output">{#each block.lines as line}<span
                class:stderr={line.stream === 'stderr'}>{line.text}{'\n'}</span
              >{/each}</pre>
        {/if}
        {#if block.truncated}
          <div class="status">{t('features.terminal.ui.truncated')}</div>
        {/if}
        {#if block.status === 'running'}
          <div class="status">{t('features.terminal.ui.running')}</div>
        {:else if block.status === 'cancelled'}
          <div class="status">{t('features.terminal.ui.cancelled')}</div>
        {:else if block.exitCode !== 0}
          <div class="status failed">
            {block.exitCode === null
              ? t('features.terminal.ui.failed')
              : t('features.terminal.ui.exit_code', { code: block.exitCode })}
          </div>
        {/if}
      </section>
    {/each}
  </div>

  <div class="input-row">
    <span class="cwd">{prompt}</span>
    <input
      bind:this={inputEl}
      bind:value={input}
      onkeydown={onKeydown}
      class="command-input"
      type="text"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      placeholder={running
        ? t('features.terminal.ui.running')
        : t('features.terminal.ui.placeholder')}
      aria-label={t('features.terminal.ui.input_label')}
    />
  </div>
</div>

<style>
  .terminal-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .scrollback {
    flex: 1;
    overflow-y: auto;
    padding: 0 var(--space-5) var(--space-3);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .hint {
    color: var(--text-tertiary);
    padding: var(--space-3) 0;
  }

  .block {
    padding: var(--space-2) 0;
  }

  .command-line {
    display: flex;
    gap: var(--space-2);
    color: var(--text-primary);
  }

  .cwd {
    color: var(--text-tertiary);
    white-space: nowrap;
  }

  .command {
    font-weight: 600;
    word-break: break-all;
  }

  .output {
    margin: var(--space-1) 0 0;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-secondary);
    font: inherit;
  }

  .output .stderr {
    color: var(--text-primary);
  }

  .status {
    margin-top: var(--space-1);
    color: var(--text-tertiary);
    font-size: var(--font-size-xs);
  }

  .status.failed {
    color: var(--text-secondary);
  }

  .input-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--border-color);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .command-input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font: inherit;
  }

  .command-input::placeholder {
    color: var(--text-tertiary);
  }
</style>
