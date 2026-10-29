<script lang="ts">
  let {
    title,
    subtitle = '',
  }: {
    title: string;
    subtitle?: string;
  } = $props();
</script>

<header class="page-header">
  <h1 class="page-title">{title}</h1>
  {#if subtitle}
    <p class="page-subtitle">{subtitle}</p>
  {/if}
  <div class="page-rule" aria-hidden="true"></div>
</header>

<style>
  .page-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .page-title {
    margin: 0;
    font-size: var(--font-size-3xl);
    font-weight: 600;
    letter-spacing: var(--tracking-display);
    line-height: 1.2;
    color: var(--text-primary);
  }

  .page-subtitle {
    margin: 0;
    font-size: var(--font-size-md);
    color: var(--text-secondary);
  }

  /* A hairline that fades out to the right: lit where the title is. */
  .page-rule {
    height: 1px;
    margin-top: var(--space-5);
    background: linear-gradient(to right, var(--accent-primary), var(--separator) 40%, transparent);
    opacity: 0.7;
  }
</style>
