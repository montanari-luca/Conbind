import { appUpdaterCheckNow } from '../../lib/ipc/updateCommands';
import { logService } from '../log/logService';

export type UpdateResult =
  | { kind: 'up-to-date' }
  | { kind: 'available'; version: string; downloadUrl: string | null }
  | { kind: 'error'; message: string }
  | { kind: 'busy' };

let inFlight = false;

export function resetUpdateCheckState(): void {
  inFlight = false;
}

export async function runUpdateCheck(): Promise<UpdateResult> {
  if (inFlight) {
    logService.debug('updateService: ignoring concurrent check request');
    return { kind: 'busy' };
  }

  inFlight = true;
  logService.info('updateService: checking for updates');

  try {
    const update = await appUpdaterCheckNow();
    if (!update) {
      logService.info('updateService: no update available');
      return { kind: 'up-to-date' };
    }
    logService.info(`updateService: update ${update.version} available`);
    return { kind: 'available', version: update.version, downloadUrl: update.downloadUrl ?? null };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    logService.error(`updateService: check failed — ${message}`);
    return { kind: 'error', message };
  } finally {
    inFlight = false;
  }
}
