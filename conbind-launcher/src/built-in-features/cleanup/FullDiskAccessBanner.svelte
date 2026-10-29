<script lang="ts">
  import { Button, WarningBanner } from '../../components';
  import { t } from '../../services/i18n';
  import { fullDiskAccess } from './fullDiskAccess.svelte';
</script>

{#if !fullDiskAccess.granted}
  <div class="fda">
    <WarningBanner>
      {t('features.cleanup.fda.message')}
      {#snippet actions()}
        <Button class="btn-secondary" onclick={() => void fullDiskAccess.openSettings()}>
          {t('features.cleanup.fda.open')}
        </Button>
      {/snippet}
    </WarningBanner>
  </div>
{/if}

<style>
  .fda {
    padding: var(--space-3) var(--space-5) 0;
  }
</style>
