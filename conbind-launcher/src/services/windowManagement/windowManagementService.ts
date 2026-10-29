import type { AppWindowInfo } from '../../bindings';
import type { WindowBounds, WindowBoundsUpdate } from '../../lib/ipc/commands';
import * as commands from '../../lib/ipc/commands';
import type {
  CapturedWorkspace,
  WindowSplitAxis,
  WorkspaceOpenReport,
  WorkspaceWindow,
} from '../../lib/ipc/commands';

export interface IWindowManagementService {
  getWindowBounds(): Promise<WindowBounds>;
  setWindowBounds(update: WindowBoundsUpdate): Promise<void>;
  setFullscreen(enable: boolean): Promise<void>;
  getMonitors(): Promise<WindowBounds[]>;
  applyPreset(presetId: string): Promise<void>;
  resize(factor: number): Promise<void>;
  minimize(): Promise<void>;
  restore(): Promise<void>;
  split(partnerId: string, axis: WindowSplitAxis): Promise<void>;
  unsplit(): Promise<boolean>;
  captureWorkspace(windowIds?: string[]): Promise<CapturedWorkspace>;
  openWorkspace(
    windows: WorkspaceWindow[],
    link: [number, number] | null,
  ): Promise<WorkspaceOpenReport>;
  previousDisplay(): Promise<void>;
  nextDisplay(): Promise<void>;
  listWindows(): Promise<AppWindowInfo[]>;
  focusWindow(id: string): Promise<void>;
  closeWindow(id: string): Promise<void>;
}

export class WindowManagementService implements IWindowManagementService {
  async getWindowBounds(): Promise<WindowBounds> {
    return commands.windowGetBounds();
  }

  async setWindowBounds(update: WindowBoundsUpdate): Promise<void> {
    return commands.windowSetBounds(update);
  }

  async setFullscreen(enable: boolean): Promise<void> {
    return commands.windowSetFullscreen(enable);
  }

  async getMonitors(): Promise<WindowBounds[]> {
    return commands.windowGetMonitors();
  }

  async applyPreset(presetId: string): Promise<void> {
    return commands.windowApplyPreset(presetId);
  }

  async resize(factor: number): Promise<void> {
    return commands.windowResize(factor);
  }

  async minimize(): Promise<void> {
    return commands.windowMinimize();
  }

  async restore(): Promise<void> {
    return commands.windowRestore();
  }

  async split(partnerId: string, axis: WindowSplitAxis): Promise<void> {
    return commands.windowSplit(partnerId, axis);
  }

  async unsplit(): Promise<boolean> {
    return commands.windowUnsplit();
  }

  async captureWorkspace(windowIds?: string[]): Promise<CapturedWorkspace> {
    return commands.windowCaptureWorkspace(windowIds);
  }

  async openWorkspace(
    windows: WorkspaceWindow[],
    link: [number, number] | null,
  ): Promise<WorkspaceOpenReport> {
    return commands.windowOpenWorkspace(windows, link);
  }

  async previousDisplay(): Promise<void> {
    return this.applyPreset('previous-display');
  }

  async nextDisplay(): Promise<void> {
    return this.applyPreset('next-display');
  }

  async listWindows(): Promise<AppWindowInfo[]> {
    return commands.windowListWindows();
  }

  async focusWindow(id: string): Promise<void> {
    return commands.windowFocusWindow(id);
  }

  async closeWindow(id: string): Promise<void> {
    return commands.windowCloseWindow(id);
  }
}

export const windowManagementService = new WindowManagementService();
