import type { Extension, ExtensionContext, IExtensionManager } from 'conbind-sdk/contracts';
import { ActionContext } from 'conbind-sdk/contracts';
import { t } from '../../services/i18n';
import { actionService, type ApplicationAction } from '../../services/action/actionService.svelte';
import AnalyzerView from './AnalyzerView.svelte';
import CleanupView from './CleanupView.svelte';
import { analyzerState } from './analyzerState.svelte';
import { runEmptyTrash, runNetworkCleanup } from './maintenance';
import { cleanupState } from './state.svelte';

const CLEANUP_VIEW_PATH = 'cleanup/CleanupView';
const ANALYZER_VIEW_PATH = 'cleanup/AnalyzerView';

const TRASH_ACTION_ID = 'cleanup:trash-selected';
const RESCAN_ACTION_ID = 'cleanup:rescan';
const REVEAL_ACTION_ID = 'cleanup:reveal';
const ENTRY_TRASH_ACTION_ID = 'cleanup:trash-entry';
const UP_ACTION_ID = 'cleanup:up';

type ActiveView = 'cleanup' | 'analyzer';

function isTextField(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA');
}

class CleanupExtension implements Extension {
  private extensionManager?: IExtensionManager;
  private activeView: ActiveView | null = null;
  private readonly handleKeydownBound = (event: KeyboardEvent) => this.handleKeydown(event);

  onUnload = () => {};

  async initialize(context: ExtensionContext): Promise<void> {
    this.extensionManager = context.getService<IExtensionManager>('extensions');
  }

  async executeCommand(commandId: string): Promise<unknown> {
    switch (commandId) {
      case 'clean-up':
      case 'developer-cleanup':
      case 'app-leftovers':
        void cleanupState.open(
          commandId === 'developer-cleanup'
            ? 'developer'
            : commandId === 'app-leftovers'
              ? 'leftovers'
              : 'general',
        );
        this.extensionManager?.navigateToView(CLEANUP_VIEW_PATH);
        return { type: 'view', viewPath: CLEANUP_VIEW_PATH };
      case 'space-analyzer':
        void analyzerState.open();
        this.extensionManager?.navigateToView(ANALYZER_VIEW_PATH);
        return { type: 'view', viewPath: ANALYZER_VIEW_PATH };
      case 'network-cleanup':
        await runNetworkCleanup();
        return undefined;
      case 'empty-trash':
        await runEmptyTrash();
        return undefined;
      default:
        throw new Error(`Unknown command: ${commandId}`);
    }
  }

  async activate(): Promise<void> {}

  async deactivate(): Promise<void> {
    this.detach();
    cleanupState.reset();
    analyzerState.reset();
  }

  async viewActivated(viewPath: string): Promise<void> {
    this.detach();
    this.activeView = viewPath === ANALYZER_VIEW_PATH ? 'analyzer' : 'cleanup';
    if (typeof window !== 'undefined') {
      window.addEventListener('keydown', this.handleKeydownBound);
    }
    if (this.activeView === 'analyzer') this.registerAnalyzerActions();
    else this.registerCleanupActions();
  }

  async viewDeactivated(_viewPath: string): Promise<void> {
    this.detach();
  }

  private detach(): void {
    this.activeView = null;
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', this.handleKeydownBound);
    }
    for (const id of [
      TRASH_ACTION_ID,
      RESCAN_ACTION_ID,
      REVEAL_ACTION_ID,
      ENTRY_TRASH_ACTION_ID,
      UP_ACTION_ID,
    ]) {
      actionService.unregisterAction(id);
    }
  }

  private registerCleanupActions(): void {
    actionService.registerAction({
      id: TRASH_ACTION_ID,
      label: t('features.cleanup.act.trash'),
      icon: 'icon:trash',
      category: t('categories.cleanup'),
      extensionId: 'cleanup',
      context: ActionContext.EXTENSION_VIEW,
      destructive: true,
      visible: () => cleanupState.totals.count > 0,
      execute: () => cleanupState.requestClean(),
    } as ApplicationAction);

    actionService.registerAction({
      id: RESCAN_ACTION_ID,
      label: t('features.cleanup.act.rescan'),
      icon: 'icon:refresh',
      category: t('categories.cleanup'),
      extensionId: 'cleanup',
      context: ActionContext.EXTENSION_VIEW,
      execute: () => cleanupState.scan(),
    } as ApplicationAction);
  }

  private registerAnalyzerActions(): void {
    actionService.registerAction({
      id: REVEAL_ACTION_ID,
      label: t('features.cleanup.act.reveal'),
      icon: 'icon:folder',
      category: t('categories.cleanup'),
      extensionId: 'cleanup',
      context: ActionContext.EXTENSION_VIEW,
      visible: () => analyzerState.selected !== null,
      execute: () => analyzerState.reveal(),
    } as ApplicationAction);

    actionService.registerAction({
      id: UP_ACTION_ID,
      label: t('features.cleanup.act.up'),
      icon: 'icon:layers',
      category: t('categories.cleanup'),
      extensionId: 'cleanup',
      context: ActionContext.EXTENSION_VIEW,
      visible: () => analyzerState.stack.length > 1,
      execute: () => analyzerState.up(),
    } as ApplicationAction);

    actionService.registerAction({
      id: ENTRY_TRASH_ACTION_ID,
      label: t('features.cleanup.act.trash_entry'),
      icon: 'icon:trash',
      category: t('categories.cleanup'),
      extensionId: 'cleanup',
      context: ActionContext.EXTENSION_VIEW,
      destructive: true,
      visible: () => analyzerState.selected !== null,
      execute: () => analyzerState.trash(),
    } as ApplicationAction);
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (this.activeView === 'analyzer') this.handleAnalyzerKey(event);
    else if (this.activeView === 'cleanup') this.handleCleanupKey(event);
  }

  private handleAnalyzerKey(event: KeyboardEvent): void {
    const consume = () => {
      event.preventDefault();
      event.stopPropagation();
    };
    switch (event.key) {
      case 'ArrowUp':
      case 'ArrowDown':
        consume();
        analyzerState.moveCursor(event.key === 'ArrowUp' ? -1 : 1);
        return;
      case 'Enter':
      case 'ArrowRight':
        if (!analyzerState.selected?.isDir) return;
        consume();
        void analyzerState.enter();
        return;
      case 'ArrowLeft':
      case 'Backspace':
        if (isTextField(event.target) || analyzerState.stack.length <= 1) return;
        consume();
        analyzerState.up();
        return;
    }
  }

  private handleCleanupKey(event: KeyboardEvent): void {
    const row = cleanupState.rows[cleanupState.cursor];
    const consume = () => {
      event.preventDefault();
      event.stopPropagation();
    };
    switch (event.key) {
      case 'ArrowUp':
      case 'ArrowDown':
        consume();
        cleanupState.moveCursor(event.key === 'ArrowUp' ? -1 : 1);
        return;
      case 'ArrowRight':
      case 'ArrowLeft': {
        if (row?.kind !== 'category') return;
        const open = cleanupState.expanded.has(row.category.category);
        if ((event.key === 'ArrowRight') === open) return;
        consume();
        cleanupState.toggleExpanded(row.category.category);
        return;
      }
      case ' ':
        // Leave Space alone while the user is typing in a text field.
        if (isTextField(event.target) || !row) return;
        consume();
        if (row.kind === 'category') cleanupState.toggleCategory(row.category);
        else cleanupState.toggleItem(row.item.id);
        return;
      case 'Enter':
        if (cleanupState.totals.count === 0) return;
        consume();
        void cleanupState.requestClean();
        return;
    }
  }
}

export default new CleanupExtension();
export { AnalyzerView, CleanupView };
