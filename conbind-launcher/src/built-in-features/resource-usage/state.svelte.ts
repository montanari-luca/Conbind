import { resourceUsageSnapshot, type ResourceSnapshot } from '../../lib/ipc/commands';

const POLL_MS = 1000;

class ResourceUsageState {
  snapshot = $state<ResourceSnapshot | null>(null);
  failed = $state(false);

  private running = false;
  private timer: ReturnType<typeof setTimeout> | null = null;

  /** Starts polling. Each poll is scheduled after the last one finishes, so slow samples never overlap. */
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
    const next = await resourceUsageSnapshot();
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

export const resourceUsageState = new ResourceUsageState();
