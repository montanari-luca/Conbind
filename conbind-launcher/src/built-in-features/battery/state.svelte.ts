import { batteryEstimate, type BatterySnapshot } from '../../lib/ipc/commands';

// The OS refines its estimate slowly, so there is no point polling faster.
const POLL_MS = 5000;

class BatteryStateStore {
  snapshot = $state<BatterySnapshot | null>(null);
  failed = $state(false);

  private running = false;
  private timer: ReturnType<typeof setTimeout> | null = null;

  /** Starts polling. Each poll is scheduled after the last one finishes, so slow reads never overlap. */
  start(): void {
    if (this.running) return;
    this.running = true;
    void this.tick();
  }

  stop(): void {
    this.running = false;
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
  }

  reset(): void {
    this.stop();
    this.snapshot = null;
    this.failed = false;
  }

  private async tick(): Promise<void> {
    const next = await batteryEstimate();
    if (!this.running) return;
    if (next) {
      this.snapshot = next;
      this.failed = false;
    } else {
      this.failed = true;
    }
    this.timer = setTimeout(() => void this.tick(), POLL_MS);
  }
}

export const batteryState = new BatteryStateStore();
