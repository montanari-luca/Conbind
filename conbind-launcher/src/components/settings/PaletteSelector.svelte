<script lang="ts">
  import { PALETTE_SWATCHES } from '../../lib/paletteSwatches';
  import { t } from '../../services/i18n';
  import type { Palette } from '../../services/theme/themeMode';

  let {
    value = 'blue-mint',
    onchange,
  }: {
    value?: Palette;
    onchange?: (v: Palette) => void;
  } = $props();

  let options = $derived<{ id: Palette; label: string }[]>([
    { id: 'blue-mint', label: t('settings.appearance.palette_blue_mint') },
    { id: 'classic', label: t('settings.appearance.palette_classic') },
  ]);
</script>

<div class="palette-selector" role="radiogroup" aria-label={t('settings.appearance.palette')}>
  {#each options as option}
    {@const swatch = PALETTE_SWATCHES[option.id]}
    <button
      class="palette-option"
      class:selected={value === option.id}
      role="radio"
      aria-checked={value === option.id}
      type="button"
      onclick={() => onchange?.(option.id)}
    >
      <span
        class="palette-preview"
        style:--sw-ground={swatch.ground}
        style:--sw-surface={swatch.surface}
        style:--sw-wash={swatch.wash}
        aria-hidden="true"
      >
        <span class="preview-side"></span>
        <span class="preview-body">
          <span class="preview-line"></span>
          <span class="preview-line short"></span>
          <span class="preview-chip"></span>
        </span>
      </span>
      <span class="palette-label">{option.label}</span>
    </button>
  {/each}
</div>

<style>
  .palette-selector {
    display: flex;
    gap: var(--space-5);
  }

  .palette-option {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    transition: color var(--dur-quick) var(--ease-travel);
  }

  .palette-option.selected {
    color: var(--text-primary);
  }

  .palette-option:focus-visible {
    outline: none;
  }

  .palette-preview {
    display: flex;
    width: var(--size-3xl);
    height: var(--size-2xl);
    padding: var(--space-1);
    gap: var(--space-1);
    border-radius: var(--radius-lg);
    background: var(--sw-ground);
    border: 2px solid transparent;
    box-shadow: var(--shadow-sm);
    transition:
      transform var(--dur-quick) var(--ease-travel),
      box-shadow var(--dur-quick) var(--ease-travel),
      border-color var(--dur-quick) var(--ease-travel);
  }

  .palette-option:hover .palette-preview {
    transform: translateY(-1px);
  }

  .palette-option.selected .palette-preview {
    border-color: var(--accent-primary);
    box-shadow: var(--glow-accent), var(--shadow-sm);
  }

  .palette-option:focus-visible .palette-preview {
    box-shadow: var(--shadow-focus);
  }

  .preview-side {
    flex: 0 0 22%;
    border-radius: var(--radius-xs);
    background: var(--sw-surface);
  }

  .preview-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  .preview-line {
    height: var(--space-1);
    border-radius: var(--radius-full);
    background: var(--sw-surface);
  }

  .preview-line.short {
    width: 60%;
  }

  .preview-chip {
    margin-top: auto;
    height: var(--space-3);
    border-radius: var(--radius-full);
    background: var(--sw-wash);
  }

  .palette-label {
    font-size: var(--font-size-2xs);
    font-family: var(--font-ui);
    font-weight: 500;
  }

  .palette-option.selected .palette-label {
    font-weight: 600;
  }

  @media (prefers-reduced-motion: reduce) {
    .palette-preview {
      transition: none;
    }
    .palette-option:hover .palette-preview {
      transform: none;
    }
  }
</style>
