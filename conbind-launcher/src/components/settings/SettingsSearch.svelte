<script module lang="ts">
  export interface SettingsSearchResult {
    key: string;
    label: string;
    description: string;
    pageLabel: string;
  }
</script>

<script lang="ts">
  import { Icon } from '..';
  import { t } from '../../services/i18n';

  let {
    query = $bindable(''),
    results,
    onpick,
  }: {
    query: string;
    results: SettingsSearchResult[];
    onpick: (key: string) => void;
  } = $props();

  let input = $state<HTMLInputElement | undefined>();
  let highlighted = $state(0);

  const searching = $derived(query.trim().length > 0);

  $effect(() => {
    // A new query restarts the cursor; reading `results` re-runs this when the
    // list changes under the same text (e.g. after a locale switch).
    void results;
    highlighted = 0;
  });

  /** Focus the field from outside (Cmd/Ctrl+F, "/"). */
  export function focus() {
    input?.focus();
    input?.select();
  }

  function clear() {
    query = '';
    input?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (query) {
        e.preventDefault();
        e.stopPropagation();
        clear();
      }
      return;
    }
    if (!searching || results.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      highlighted = (highlighted + 1) % results.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      highlighted = (highlighted - 1 + results.length) % results.length;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      onpick(results[highlighted].key);
    }
  }
</script>

<div class="settings-search">
  <div class="search-field">
    <span class="search-icon"><Icon name="search" size={14} /></span>
    <input
      bind:this={input}
      bind:value={query}
      {onkeydown}
      class="search-input-field"
      type="text"
      role="combobox"
      aria-expanded={searching}
      aria-controls="settings-search-results"
      aria-autocomplete="list"
      autocomplete="off"
      spellcheck="false"
      placeholder={t('settings.search.placeholder')}
      aria-label={t('settings.search.placeholder')}
    />
    {#if query}
      <button
        class="search-clear"
        type="button"
        aria-label={t('settings.search.clear')}
        onclick={clear}
      >
        <Icon name="plus" size={12} />
      </button>
    {/if}
  </div>

  {#if searching}
    <ul
      id="settings-search-results"
      class="search-results custom-scrollbar"
      role="listbox"
      aria-label={t('settings.search.results')}
    >
      {#each results as result, i (result.key)}
        <li role="presentation">
          <button
            class="search-result"
            class:highlighted={i === highlighted}
            type="button"
            role="option"
            aria-selected={i === highlighted}
            onmouseenter={() => (highlighted = i)}
            onclick={() => onpick(result.key)}
          >
            <span class="result-label">{result.label}</span>
            <span class="result-meta">{result.pageLabel}</span>
          </button>
        </li>
      {:else}
        <li class="search-empty">{t('settings.search.no_results')}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .settings-search {
    display: flex;
    flex-direction: column;
    min-height: 0;
    gap: var(--space-3);
  }

  .search-field {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    inset-inline-start: var(--space-4);
    display: flex;
    color: var(--text-tertiary);
    pointer-events: none;
    transition: color var(--dur-quick) var(--ease-travel);
  }

  .search-field:focus-within .search-icon {
    color: var(--accent-primary);
  }

  .search-input-field {
    width: 100%;
    padding: var(--space-3) var(--space-8) var(--space-3) var(--space-8);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: var(--font-size-md);
    outline: none;
    transition:
      border-color var(--dur-quick) var(--ease-travel),
      box-shadow var(--dur-quick) var(--ease-travel);
  }

  .search-input-field::placeholder {
    color: var(--text-tertiary);
  }

  .search-input-field:focus {
    border-color: var(--accent-primary);
    box-shadow: var(--shadow-focus);
  }

  .search-clear {
    position: absolute;
    inset-inline-end: var(--space-3);
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--size-md);
    height: var(--size-md);
    border: none;
    border-radius: var(--radius-full);
    background: var(--bg-hover);
    color: var(--text-secondary);
    cursor: pointer;
    transform: rotate(45deg);
  }

  .search-clear:hover {
    color: var(--text-primary);
  }

  .search-results {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
  }

  .search-result {
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-primary);
    text-align: start;
    cursor: pointer;
    font-family: var(--font-ui);
  }

  .search-result.highlighted {
    background: var(--bg-selected);
  }

  .result-label {
    font-size: var(--font-size-md);
    font-weight: 500;
  }

  .result-meta {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .search-empty {
    padding: var(--space-5) var(--space-4);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }
</style>
