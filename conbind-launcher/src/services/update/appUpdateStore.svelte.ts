import { type UnlistenFn } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import { logService } from '../log/logService';
import { appUpdaterGetPending } from '../../lib/ipc/updateCommands';
import { appListen } from '../../lib/ipc/bridgeEvents';

export type AppUpdatePhase = 'idle' | 'checking' | 'ready' | 'error';

/** Where to send the user when the release feed has no direct link for their platform yet. */
export const UPDATE_FALLBACK_URL = 'https://github.com/montanari-luca/MintFlow-Releases';

// Svelte 5 class-based reactive state — $state is only valid as a class field or
// variable declaration initializer, not inside an object literal.
class AppUpdateStore {
  phase = $state<AppUpdatePhase>('idle');
  pendingVersion = $state<string | null>(null);
  downloadUrl = $state<string | null>(null);
  errorMessage = $state<string | null>(null);
}

export const appUpdateState = new AppUpdateStore();

export function markUpdateAvailable(update: {
  version: string;
  downloadUrl?: string | null;
}): void {
  appUpdateState.phase = 'ready';
  appUpdateState.pendingVersion = update.version;
  appUpdateState.downloadUrl = update.downloadUrl ?? null;
}

/** Open the installer download for the available update in the user's browser. */
export async function openUpdateDownload(): Promise<void> {
  try {
    await openUrl(appUpdateState.downloadUrl ?? UPDATE_FALLBACK_URL);
  } catch (e) {
    logService.warn(`appUpdateStore: failed to open update download — ${e}`);
  }
}

export async function refreshPendingUpdate(): Promise<void> {
  try {
    const pending = await appUpdaterGetPending();
    if (pending) markUpdateAvailable(pending);
  } catch (e) {
    logService.warn(`appUpdateStore: failed to get pending update — ${e}`);
  }
}

let unlisteners: UnlistenFn[] = [];
let initialized = false;

export async function initAppUpdateStore(): Promise<void> {
  if (initialized) return;
  initialized = true;

  // Restore state from Rust in case a check completed before the webview loaded
  await refreshPendingUpdate();

  // Listen to Rust-emitted events
  const checking = await appListen('conbind:app-update:checking', () => {
    appUpdateState.phase = 'checking';
    appUpdateState.errorMessage = null;
  });

  const idle = await appListen('conbind:app-update:idle', () => {
    appUpdateState.phase = 'idle';
    appUpdateState.pendingVersion = null;
    appUpdateState.downloadUrl = null;
  });

  const ready = await appListen<{ version: string; downloadUrl?: string | null }>(
    'conbind:app-update:ready',
    (e) => {
      markUpdateAvailable(e.payload);
      logService.info(`appUpdateStore: update ${e.payload.version} available`);
    },
  );

  const error = await appListen<{ message: string }>('conbind:app-update:error', (e) => {
    appUpdateState.phase = 'error';
    appUpdateState.errorMessage = e.payload.message;
    logService.warn(`appUpdateStore: auto-check error — ${e.payload.message}`);
  });

  unlisteners = [checking, idle, ready, error];
}

export function destroyAppUpdateStore(): void {
  for (const unlisten of unlisteners) unlisten();
  unlisteners = [];
  initialized = false;
  appUpdateState.phase = 'idle';
  appUpdateState.pendingVersion = null;
  appUpdateState.downloadUrl = null;
  appUpdateState.errorMessage = null;
}
