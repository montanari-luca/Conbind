import type { Extension, ExtensionContext, IExtensionManager } from 'conbind-sdk/contracts';
import { batteryState } from './state.svelte';
import BatteryView from './BatteryView.svelte';

const VIEW_PATH = 'battery/BatteryView';

class BatteryExtension implements Extension {
  onUnload = () => {};
  private extensionManager?: IExtensionManager;

  async initialize(context: ExtensionContext): Promise<void> {
    this.extensionManager = context.getService<IExtensionManager>('extensions');
  }

  async executeCommand(commandId: string): Promise<any> {
    if (commandId === 'estimate-battery') {
      this.extensionManager?.navigateToView(VIEW_PATH);
      return { type: 'view', viewPath: VIEW_PATH };
    }
    throw new Error(`Unknown command: ${commandId}`);
  }

  async activate(): Promise<void> {}

  async deactivate(): Promise<void> {
    batteryState.reset();
  }
}

export default new BatteryExtension();
export { BatteryView };
