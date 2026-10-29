import { invokeSafe } from './invokeSafe';

// ── Resource usage (CPU, RAM, network, disk, GPU) ─────────────────────────────

export interface ResourceCpu {
  percent: number;
  cores: number;
}

export interface ResourceMemory {
  used: number;
  total: number;
  swapUsed: number;
  swapTotal: number;
}

export interface ResourceNetwork {
  rxPerSec: number;
  txPerSec: number;
  totalRx: number;
  totalTx: number;
}

export interface ResourceDisk {
  name: string;
  mount: string;
  used: number;
  total: number;
}

/** Any figure the platform cannot report is `null`. */
export interface ResourceGpu {
  name: string;
  usagePercent: number | null;
  vramUsed: number | null;
  vramTotal: number | null;
}

export interface ResourceSnapshot {
  cpu: ResourceCpu;
  memory: ResourceMemory;
  network: ResourceNetwork;
  disks: ResourceDisk[];
  gpus: ResourceGpu[];
}

export async function resourceUsageSnapshot(): Promise<ResourceSnapshot | null> {
  return invokeSafe<ResourceSnapshot | null>('resource_usage_snapshot', undefined, {
    silent: true,
  });
}
