import { describe, it, expect } from 'vitest';
import { formatPercent, formatRate, formatSize, fraction, vramFraction } from './resourceLogic';

describe('resourceLogic', () => {
  it('formats sizes in decimal units', () => {
    expect(formatSize(512)).toBe('512 B');
    expect(formatSize(1500)).toBe('1.5 KB');
    expect(formatSize(16_000_000_000)).toBe('16.0 GB');
    expect(formatSize(250_000_000_000)).toBe('250 GB');
  });

  it('formats rates per second', () => {
    expect(formatRate(2_500_000)).toBe('2.5 MB/s');
  });

  it('clamps fractions and tolerates an empty total', () => {
    expect(fraction(50, 100)).toBe(0.5);
    expect(fraction(150, 100)).toBe(1);
    expect(fraction(-5, 100)).toBe(0);
    expect(fraction(5, 0)).toBe(0);
  });

  it('rounds and clamps percentages', () => {
    expect(formatPercent(32.4)).toBe('32%');
    expect(formatPercent(140)).toBe('100%');
    expect(formatPercent(-1)).toBe('0%');
  });

  it('reports VRAM only when both figures are known', () => {
    const base = { name: 'g', usagePercent: null };
    expect(vramFraction({ ...base, vramUsed: 1, vramTotal: 4 })).toBe(0.25);
    expect(vramFraction({ ...base, vramUsed: null, vramTotal: 4 })).toBeNull();
    expect(vramFraction({ ...base, vramUsed: 1, vramTotal: null })).toBeNull();
  });
});
