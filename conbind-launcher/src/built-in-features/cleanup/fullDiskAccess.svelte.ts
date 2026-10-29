import { cleanupFullDiskAccess, cleanupOpenFullDiskAccess } from '../../lib/ipc/commands';

/** Whether Conbind can read protected folders, so views can warn about under-counted sizes. */
class FullDiskAccessState {
  granted = $state(true);

  async check(): Promise<void> {
    try {
      this.granted = await cleanupFullDiskAccess();
    } catch {
      // The warning is a hint; failing to probe shouldn't show one.
      this.granted = true;
    }
  }

  async openSettings(): Promise<void> {
    await cleanupOpenFullDiskAccess();
  }
}

export const fullDiskAccess = new FullDiskAccessState();
