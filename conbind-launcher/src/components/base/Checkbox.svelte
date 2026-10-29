<script lang="ts">
  let {
    checked = false,
    disabled = false,
    onchange,
  }: {
    checked?: boolean;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
  } = $props();
</script>

<label class="checkbox-wrapper" class:disabled>
  <input
    type="checkbox"
    class="sr-only"
    {checked}
    {disabled}
    onchange={(e) => onchange?.((e.target as HTMLInputElement).checked)}
  />
  <span class="checkbox-box" class:checked>
    <svg viewBox="0 0 12 10" fill="none" xmlns="http://www.w3.org/2000/svg" class="checkmark">
      <path
        d="M1.5 5.2L4.5 8.2L10.5 1.8"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        pathLength="1"
      />
    </svg>
  </span>
</label>

<style>
  .checkbox-wrapper {
    display: inline-flex;
    align-items: center;
    cursor: pointer;
    user-select: none;
  }

  .checkbox-wrapper.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: calc(-1 * var(--space-0-5));
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  .checkbox-box {
    position: relative;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
    border: 1.5px solid var(--border-color);
    background: var(--bg-primary);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.08);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition:
      background var(--transition-normal),
      border-color var(--transition-normal),
      box-shadow var(--transition-normal),
      transform var(--transition-fast);
  }

  .checkbox-box.checked {
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--accent-primary-fill) 82%, white) 0%,
      var(--accent-primary-fill) 100%
    );
    border-color: color-mix(in srgb, var(--accent-primary-fill) 85%, black);
    color: var(--text-on-accent);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.25),
      0 1px 3px color-mix(in srgb, var(--accent-primary-fill) 40%, transparent);
  }

  .checkbox-wrapper:not(.disabled):hover .checkbox-box:not(.checked) {
    border-color: var(--text-secondary);
    background: var(--bg-hover);
  }

  .checkbox-wrapper:not(.disabled):active .checkbox-box {
    transform: scale(0.92);
  }

  .checkbox-wrapper .sr-only:focus-visible ~ .checkbox-box {
    box-shadow: var(--shadow-focus);
  }

  /* The tick draws itself in rather than popping. pathLength=1 on the path
     makes the dash maths independent of the glyph's real length. */
  .checkmark {
    width: 12px;
    height: 10px;
  }

  .checkmark path {
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    transition: stroke-dashoffset var(--transition-smooth);
  }

  .checkbox-box.checked .checkmark path {
    stroke-dashoffset: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .checkbox-box,
    .checkmark path {
      transition: none;
    }
  }
</style>
