import {
  registerBuiltinSearchProvider,
  type BuiltinSearchRow,
} from '../../services/search/builtinSearchProviders';
import { t } from '../../services/i18n';
import type { Extension, ExtensionContext, IExtensionManager } from 'conbind-sdk/contracts';
import { terminalState } from './terminalState.svelte';
import { parseTerminalQuery } from './terminalLogic';
import TerminalView from './TerminalView.svelte';

const VIEW_PATH = 'terminal/TerminalView';
const RUN_ROW_ID = 'terminal_run';

class TerminalExtension implements Extension {
  onUnload = () => {};
  private extensionManager?: IExtensionManager;
  private enabled = true;

  async initialize(context: ExtensionContext): Promise<void> {
    this.extensionManager = context.getService<IExtensionManager>('extensions');
  }

  async executeCommand(commandId: string): Promise<any> {
    if (commandId === 'open-terminal') {
      this.extensionManager?.navigateToView(VIEW_PATH);
      return { type: 'view', viewPath: VIEW_PATH };
    }
    throw new Error(`Unknown command: ${commandId}`);
  }

  async activate(): Promise<void> {
    this.enabled = true;
  }

  // The session outlives the view, so leaving it keeps the scrollback.
  async deactivate(): Promise<void> {
    this.enabled = false;
  }

  async searchRows(query: string): Promise<BuiltinSearchRow[]> {
    if (!this.enabled) return [];
    const command = parseTerminalQuery(query);
    if (!command) return [];
    return [
      {
        id: RUN_ROW_ID,
        score: 1.0,
        title: t('features.terminal.run', { command }),
        icon: 'icon:terminal',
        priority: 'top',
        primaryActionLabel: t('features.terminal.run_action'),
        actionPayload: { command },
      },
    ];
  }

  async executeSearchResult(_rowId: string, payload?: unknown): Promise<void> {
    const command = (payload as { command?: unknown } | undefined)?.command;
    if (typeof command !== 'string' || !command) return;
    terminalState.queue(command);
    this.extensionManager?.navigateToView(VIEW_PATH);
  }
}

const extension = new TerminalExtension();
registerBuiltinSearchProvider({
  extensionId: 'terminal',
  search: (query) => extension.searchRows(query),
  execute: (rowId, payload) => extension.executeSearchResult(rowId, payload),
});
export default extension;
export { TerminalView };
