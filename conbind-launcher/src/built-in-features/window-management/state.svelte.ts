import { logService } from '../../services/log/logService';
import type { IStorageService } from 'conbind-sdk/contracts';
import type {
  CapturedWorkspace,
  WindowBounds,
  WindowSplitAxis,
  WorkspaceWindow,
} from '../../lib/ipc/commands';
import { useListSelection } from '../../lib/listSelection.svelte';

export interface CustomLayout {
  id: string;
  name: string;
  bounds: WindowBounds;
}

export interface Workspace {
  id: string;
  name: string;
  windows: WorkspaceWindow[];
  /** Indices into `windows` of a pair to re-link when the workspace is opened. */
  link: [number, number] | null;
  created: number;
  updated: number;
}

export const MAX_WORKSPACE_NAME = 40;

const STORAGE_KEY_LAYOUTS = 'custom_layouts';
const STORAGE_KEY_WORKSPACES = 'workspaces';
const STORAGE_KEY_ACTIVE_WORKSPACE = 'active_workspace';

/** Returns the trimmed name, or null when it is empty or longer than the limit. */
export function cleanWorkspaceName(name: string): string | null {
  const trimmed = name.trim();
  if (!trimmed || [...trimmed].length > MAX_WORKSPACE_NAME) return null;
  return trimmed;
}

export class WindowManagementState {
  customLayouts = $state<CustomLayout[]>([]);
  store = $state<IStorageService | null>(null);
  /** Which way the split picker will tile the pair; set by the command that opens it. */
  splitAxis = $state<WindowSplitAxis>('side');

  workspaces = $state<Workspace[]>([]);
  /** The workspace opened or saved last; `update-workspace` defaults to it. */
  activeWorkspaceId = $state<string | null>(null);

  private selection = useListSelection({ items: () => this.customLayouts });
  private workspaceSelection = useListSelection({ items: () => this.workspaces });

  get selectedIndex(): number {
    return this.selection.selectedIndex;
  }

  get selectedLayout(): CustomLayout | null {
    return this.selection.selectedItem;
  }

  get selectedWorkspaceIndex(): number {
    return this.workspaceSelection.selectedIndex;
  }

  get selectedWorkspace(): Workspace | null {
    return this.workspaceSelection.selectedItem;
  }

  get activeWorkspace(): Workspace | null {
    return this.workspaces.find((w) => w.id === this.activeWorkspaceId) ?? null;
  }

  setWorkspaceIndex(index: number): void {
    this.workspaceSelection.setIndex(index);
  }

  moveWorkspaceSelection(direction: 'up' | 'down'): void {
    this.workspaceSelection.moveSelection(direction);
  }

  setStore(store: IStorageService): void {
    this.store = store;
  }

  setIndex(index: number): void {
    this.selection.setIndex(index);
  }

  moveSelection(direction: 'up' | 'down'): void {
    this.selection.moveSelection(direction);
  }

  async loadFromStorage(store?: IStorageService): Promise<void> {
    const targetStore = store ?? this.store;
    if (!targetStore) return;
    try {
      const rawLayouts = await targetStore.get(STORAGE_KEY_LAYOUTS);
      this.customLayouts = rawLayouts ? JSON.parse(rawLayouts) : [];
    } catch {
      logService.warn('[WindowManagement] Failed to parse custom_layouts — resetting to []');
      this.customLayouts = [];
    }

    try {
      const rawWorkspaces = await targetStore.get(STORAGE_KEY_WORKSPACES);
      this.workspaces = rawWorkspaces ? JSON.parse(rawWorkspaces) : [];
    } catch {
      logService.warn('[WindowManagement] Failed to parse workspaces — resetting to []');
      this.workspaces = [];
    }

    try {
      this.activeWorkspaceId = (await targetStore.get(STORAGE_KEY_ACTIVE_WORKSPACE)) ?? null;
    } catch {
      this.activeWorkspaceId = null;
    }
  }

  async addCustomLayout(
    name: string,
    bounds: WindowBounds,
    store?: IStorageService,
  ): Promise<CustomLayout> {
    const targetStore = store ?? this.store;
    const layout: CustomLayout = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
      name,
      bounds,
    };
    this.customLayouts = [...this.customLayouts, layout];
    if (targetStore) {
      await targetStore.set(STORAGE_KEY_LAYOUTS, JSON.stringify(this.customLayouts));
    }
    this.selection.setIndex(this.customLayouts.length - 1);
    return layout;
  }

  async deleteCustomLayout(id: string, store?: IStorageService): Promise<void> {
    const targetStore = store ?? this.store;
    this.customLayouts = this.customLayouts.filter((l) => l.id !== id);
    if (targetStore) {
      await targetStore.set(STORAGE_KEY_LAYOUTS, JSON.stringify(this.customLayouts));
    }
  }

  async renameCustomLayout(id: string, newName: string, store?: IStorageService): Promise<void> {
    const targetStore = store ?? this.store;
    const trimmed = newName.trim();
    if (!trimmed) return;
    const existing = this.customLayouts.find((l) => l.id === id);
    if (!existing || existing.name === trimmed) return;

    this.customLayouts = this.customLayouts.map((l) => (l.id === id ? { ...l, name: trimmed } : l));
    if (targetStore) {
      await targetStore.set(STORAGE_KEY_LAYOUTS, JSON.stringify(this.customLayouts));
    }
  }

  findWorkspaceByName(name: string): Workspace | undefined {
    const key = name.trim().toLowerCase();
    return this.workspaces.find((w) => w.name.toLowerCase() === key);
  }

  /** "Workspace 1", "Workspace 2", ... the first one not taken. */
  nextWorkspaceName(): string {
    for (let n = 1; ; n++) {
      const candidate = `Workspace ${n}`;
      if (!this.findWorkspaceByName(candidate)) return candidate;
    }
  }

  private async persistWorkspaces(store?: IStorageService): Promise<void> {
    const targetStore = store ?? this.store;
    if (targetStore) {
      await targetStore.set(STORAGE_KEY_WORKSPACES, JSON.stringify(this.workspaces));
    }
  }

  async setActiveWorkspace(id: string | null, store?: IStorageService): Promise<void> {
    const targetStore = store ?? this.store;
    this.activeWorkspaceId = id;
    if (targetStore) {
      if (id) await targetStore.set(STORAGE_KEY_ACTIVE_WORKSPACE, id);
      else await targetStore.delete(STORAGE_KEY_ACTIVE_WORKSPACE);
    }
  }

  async addWorkspace(
    name: string,
    captured: CapturedWorkspace,
    store?: IStorageService,
  ): Promise<Workspace> {
    const cleaned = cleanWorkspaceName(name);
    if (!cleaned) throw new Error(`Workspace names must be 1 to ${MAX_WORKSPACE_NAME} characters`);
    if (this.findWorkspaceByName(cleaned)) {
      throw new Error(`A workspace named "${cleaned}" already exists`);
    }
    const now = Date.now();
    const workspace: Workspace = {
      id: `${now}-${Math.random().toString(36).slice(2)}`,
      name: cleaned,
      windows: captured.windows,
      link: captured.link,
      created: now,
      updated: now,
    };
    this.workspaces = [...this.workspaces, workspace];
    await this.persistWorkspaces(store);
    this.workspaceSelection.setIndex(this.workspaces.length - 1);
    await this.setActiveWorkspace(workspace.id, store);
    return workspace;
  }

  /** Replaces a workspace's windows with a fresh capture, keeping its name and id. */
  async replaceWorkspaceWindows(
    id: string,
    captured: CapturedWorkspace,
    store?: IStorageService,
  ): Promise<Workspace | undefined> {
    const existing = this.workspaces.find((w) => w.id === id);
    if (!existing) return undefined;
    const updated: Workspace = {
      ...existing,
      windows: captured.windows,
      link: captured.link,
      updated: Date.now(),
    };
    this.workspaces = this.workspaces.map((w) => (w.id === id ? updated : w));
    await this.persistWorkspaces(store);
    await this.setActiveWorkspace(id, store);
    return updated;
  }

  async renameWorkspace(id: string, newName: string, store?: IStorageService): Promise<void> {
    const cleaned = cleanWorkspaceName(newName);
    if (!cleaned) throw new Error(`Workspace names must be 1 to ${MAX_WORKSPACE_NAME} characters`);
    const existing = this.workspaces.find((w) => w.id === id);
    if (!existing || existing.name === cleaned) return;
    const clash = this.findWorkspaceByName(cleaned);
    if (clash && clash.id !== id) {
      throw new Error(`A workspace named "${cleaned}" already exists`);
    }
    this.workspaces = this.workspaces.map((w) => (w.id === id ? { ...w, name: cleaned } : w));
    await this.persistWorkspaces(store);
  }

  async deleteWorkspace(id: string, store?: IStorageService): Promise<void> {
    this.workspaces = this.workspaces.filter((w) => w.id !== id);
    await this.persistWorkspaces(store);
    if (this.activeWorkspaceId === id) {
      await this.setActiveWorkspace(null, store);
    }
  }

  reset(): void {
    this.selection.setIndex(0);
    this.workspaceSelection.setIndex(0);
  }
}

export const windowManagementState = new WindowManagementState();
