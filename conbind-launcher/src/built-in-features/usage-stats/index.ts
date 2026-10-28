import type {
  Extension,
  ExtensionContext,
  IExtensionManager,
  ILogService,
} from 'conbind-sdk/contracts';
import { usageStatsState } from './usageStatsState.svelte';
import DefaultView from './DefaultView.svelte';

const VIEW_PATH = 'usage-stats/DefaultView';

class UsageStatsExtension implements Extension {
  onUnload = () => {};
  private logService?: ILogService;
  private extensionManager?: IExtensionManager;

  async initialize(context: ExtensionContext): Promise<void> {
    this.logService = context.getService<ILogService>('log');
    this.extensionManager = context.getService<IExtensionManager>('extensions');
  }

  async executeCommand(commandId: string): Promise<any> {
    if (commandId === 'open-usage-stats') {
      await usageStatsState.load();
      this.extensionManager?.navigateToView(VIEW_PATH);
      return { type: 'view', viewPath: VIEW_PATH };
    }
    throw new Error(`Unknown command: ${commandId}`);
  }

  async activate(): Promise<void> {
    await usageStatsState.load();
  }

  async deactivate(): Promise<void> {
    usageStatsState.reset();
  }
}

export default new UsageStatsExtension();
export { DefaultView };
