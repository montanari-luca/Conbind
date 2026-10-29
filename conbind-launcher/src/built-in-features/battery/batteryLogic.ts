import type { BatterySnapshot } from '../../lib/ipc/commands';

export type BatteryView =
  | { kind: 'no-battery' }
  | { kind: 'time-left'; seconds: number }
  | { kind: 'time-to-full'; seconds: number }
  | { kind: 'calculating' }
  | { kind: 'charging' }
  | { kind: 'charged' }
  | { kind: 'plugged' };

export interface Duration {
  hours: number;
  minutes: number;
}

/** Whole hours and minutes; anything under a minute still reads as one minute. */
export function splitDuration(seconds: number): Duration {
  const totalMinutes = Math.max(1, Math.round(seconds / 60));
  return { hours: Math.floor(totalMinutes / 60), minutes: totalMinutes % 60 };
}

/** Decides which message the view shows for a reading. */
export function deriveView(snap: BatterySnapshot): BatteryView {
  if (!snap.present) return { kind: 'no-battery' };
  switch (snap.state) {
    case 'discharging':
      return snap.secondsToEmpty !== null
        ? { kind: 'time-left', seconds: snap.secondsToEmpty }
        : { kind: 'calculating' };
    case 'charging':
      return snap.secondsToFull !== null
        ? { kind: 'time-to-full', seconds: snap.secondsToFull }
        : { kind: 'charging' };
    case 'charged':
      return { kind: 'charged' };
    default:
      return { kind: 'plugged' };
  }
}
