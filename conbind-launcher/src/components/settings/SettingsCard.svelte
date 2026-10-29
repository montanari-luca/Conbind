<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="settings-card">
  {@render children()}
</div>

<style>
  .settings-card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-xl);
    overflow: hidden;
    transition:
      border-color var(--dur-quick) var(--ease-travel),
      box-shadow var(--dur-travel) var(--ease-travel);
  }

  /* A faint lit edge when the pointer or keyboard is inside the card. Under
     the Classic palette --glow-accent-soft is empty, so only the border moves. */
  .settings-card:hover,
  .settings-card:focus-within {
    border-color: color-mix(in srgb, var(--accent-primary) 32%, var(--border-color));
    box-shadow: var(--glow-accent-soft);
  }

  @media (prefers-reduced-motion: reduce) {
    .settings-card {
      transition: none;
    }
  }
</style>
