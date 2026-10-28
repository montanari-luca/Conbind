import { logService } from '../../services/log/logService';
import { invokeSafe } from './invokeSafe';

// ── Usage insights ────────────────────────────────────────────────────────────

export interface UsageTopItem {
  id: string;
  label?: string | null;
  count: number;
}
export interface UsageStats {
  activeDays: number;
  totalLaunches: number;
  top: UsageTopItem[];
}
export async function getUsageStats(): Promise<UsageStats> {
  return invokeSafe('get_usage_stats');
}
export async function recordActiveDay(): Promise<void> {
  try {
    await invokeSafe('record_active_day', undefined, { silent: true });
  } catch (err) {
    logService.debug(`[usage] recordActiveDay failed silently: ${err}`);
  }
}
/** Local daily compatibility counts. */
export async function recordWorkerFallback(extensionId: string): Promise<void> {
  try {
    await invokeSafe('record_worker_fallback', { extensionId }, { silent: true });
  } catch (err) {
    logService.debug(`[usage] recordWorkerFallback failed silently: ${err}`);
  }
}
