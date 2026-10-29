import type { Extension, ExtensionContext, IExtensionManager } from 'conbind-sdk/contracts';
import { resourceUsageState } from './state.svelte';
import ResourceUsageView from './ResourceUsageView.svelte';

const VIEW_PATH = 'resource-usage/ResourceUsageView';

class ResourceUsageExtension implements Extension {
  onUnload = () => {};
  private extensionManager?: IExtensionManager;

  async initialize(context: ExtensionContext): Promise<void> {
    this.extensionManager = context.getService<IExtensionManager>('extensions');
  }

  async executeCommand(commandId: string): Promise<any> {
    if (commandId === 'view-resource-usage') {
      this.extensionManager?.navigateToView(VIEW_PATH);
      return { type: 'view', viewPath: VIEW_PATH };
    }
    throw new Error(`Unknown command: ${commandId}`);
  }

  async activate(): Promise<void> {}

  async deactivate(): Promise<void> {
    resourceUsageState.reset();
  }
}

export default new ResourceUsageExtension();
export { ResourceUsageView };
