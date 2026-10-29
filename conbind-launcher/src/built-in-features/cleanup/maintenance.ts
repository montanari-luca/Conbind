import { cleanupEmptyTrash, cleanupNetwork, setFocusLock } from '../../lib/ipc/commands';
import { feedbackService } from '../../services/feedback/feedbackService.svelte';
import { t } from '../../services/i18n';

/** Confirm dialog that keeps the launcher open while it is showing. */
async function confirm(title: string, message: string, confirmText: string): Promise<boolean> {
  await setFocusLock(true);
  try {
    return await feedbackService.confirmAlert({
      title,
      message,
      confirmText,
      cancelText: t('features.cleanup.confirm.cancel'),
      variant: 'danger',
    });
  } finally {
    await setFocusLock(false);
  }
}

export async function runNetworkCleanup(): Promise<void> {
  const ok = await confirm(
    t('features.cleanup.network.title'),
    t('features.cleanup.network.message'),
    t('features.cleanup.network.confirm'),
  );
  if (!ok) return;
  // A dismissed OS password prompt is the user changing their mind, not a failure.
  if ((await cleanupNetwork()) === 'done') {
    await feedbackService.showHUD(t('features.cleanup.network.done'));
  }
}

export async function runEmptyTrash(): Promise<void> {
  const ok = await confirm(
    t('features.cleanup.empty_trash.title'),
    t('features.cleanup.empty_trash.message'),
    t('features.cleanup.empty_trash.confirm'),
  );
  if (!ok) return;
  await cleanupEmptyTrash();
  await feedbackService.showHUD(t('features.cleanup.empty_trash.done'));
}
