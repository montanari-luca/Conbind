import { describe, it, expect } from 'vitest';
import type { BatterySnapshot } from '../../lib/ipc/commands';
import { deriveView, splitDuration } from './batteryLogic';

const base: BatterySnapshot = {
  present: true,
  percent: 64,
  state: 'discharging',
  secondsToEmpty: null,
  secondsToFull: null,
};

describe('splitDuration', () => {
  it('splits seconds into hours and minutes', () => {
    expect(splitDuration(11_520)).toEqual({ hours: 3, minutes: 12 });
    expect(splitDuration(45 * 60)).toEqual({ hours: 0, minutes: 45 });
  });

  it('rounds to the nearest minute and never shows zero', () => {
    expect(splitDuration(89)).toEqual({ hours: 0, minutes: 1 });
    expect(splitDuration(5)).toEqual({ hours: 0, minutes: 1 });
    expect(splitDuration(3599)).toEqual({ hours: 1, minutes: 0 });
  });
});

describe('deriveView', () => {
  it('reports a machine without a battery', () => {
    expect(deriveView({ ...base, present: false })).toEqual({ kind: 'no-battery' });
  });

  it('shows time left while discharging', () => {
    expect(deriveView({ ...base, secondsToEmpty: 7200 })).toEqual({
      kind: 'time-left',
      seconds: 7200,
    });
  });

  it('says calculating while discharging without an estimate', () => {
    expect(deriveView(base)).toEqual({ kind: 'calculating' });
  });

  it('shows time to full while charging, or plain charging without one', () => {
    expect(deriveView({ ...base, state: 'charging', secondsToFull: 1800 })).toEqual({
      kind: 'time-to-full',
      seconds: 1800,
    });
    expect(deriveView({ ...base, state: 'charging' })).toEqual({ kind: 'charging' });
  });

  it('handles fully charged and unknown states', () => {
    expect(deriveView({ ...base, state: 'charged' })).toEqual({ kind: 'charged' });
    expect(deriveView({ ...base, state: 'unknown' })).toEqual({ kind: 'plugged' });
  });
});
