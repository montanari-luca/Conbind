import { invokeSafe } from './invokeSafe';

// ── Battery estimate ──────────────────────────────────────────────────────────

export type BatteryState = 'discharging' | 'charging' | 'charged' | 'unknown';

/** Any figure the platform cannot report is `null`; `present` is false on machines without a battery. */
export interface BatterySnapshot {
  present: boolean;
  percent: number | null;
  state: BatteryState;
  secondsToEmpty: number | null;
  secondsToFull: number | null;
}

export async function batteryEstimate(): Promise<BatterySnapshot | null> {
  return invokeSafe<BatterySnapshot | null>('battery_estimate', undefined, { silent: true });
}
