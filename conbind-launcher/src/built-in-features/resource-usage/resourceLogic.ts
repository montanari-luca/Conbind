import type { ResourceGpu } from '../../lib/ipc/commands';

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** Decimal byte formatting, matching the rest of the launcher. */
export function formatSize(bytes: number): string {
  if (bytes < 1000) return `${Math.round(bytes)} B`;
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit++;
  }
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${UNITS[unit]}`;
}

export function formatRate(bytesPerSec: number): string {
  return `${formatSize(bytesPerSec)}/s`;
}

/** Fill fraction clamped to 0..1; an empty total reads as 0. */
export function fraction(used: number, total: number): number {
  if (!(total > 0)) return 0;
  return Math.max(0, Math.min(1, used / total));
}

export function formatPercent(percent: number): string {
  return `${Math.round(Math.max(0, Math.min(100, percent)))}%`;
}

/** `null` when the GPU did not report memory, so the card can say so. */
export function vramFraction(gpu: ResourceGpu): number | null {
  if (gpu.vramUsed === null || gpu.vramTotal === null) return null;
  return fraction(gpu.vramUsed, gpu.vramTotal);
}
