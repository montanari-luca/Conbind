// conbind-launcher/src/lib/ipc/windowCommands.ts
// Tauri command wrappers, re-exported through ./commands (the barrel).
import { invokeSafe } from './invokeSafe';

async function emitExtensionDeeplink(extensionId: string, commandId: string): Promise<void> {
  const { emit } = await import('@tauri-apps/api/event');
  await emit('conbind:deeplink:extension', { extensionId, commandId, args: {} });
}

/** Request the main launcher window to open the built-in Store view. */
export async function openStore(): Promise<void> {
  await emitExtensionDeeplink('store', 'browse');
}

/** Request the main launcher window to open the AI extension builder. */
export async function openCreateExtensionWithAi(): Promise<void> {
  await emitExtensionDeeplink('create-extension', 'build-with-ai');
}

/** Request the main launcher window to open the extension scaffolder. */
export async function openCreateExtension(): Promise<void> {
  await emitExtensionDeeplink('create-extension', 'open');
}
import type { AppWindowInfo, LauncherPlacement } from '../../bindings';

// ── Window ────────────────────────────────────────────────────────────────────

/** Wait two rAFs: long enough for the webview to commit at least one fresh
 * layer-tree / paint after the render process transitions back to visible. */
function twoFrames(): Promise<void> {
  return new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
  );
}

/**
 * Reveal the launcher window. When the panel was hidden, uses a two-phase
 * reveal (prepare at alpha 0 / off-screen, wait for the webview to push a
 * fresh frame, then commit to its final position) so the user doesn't see
 * the stale cached composite from the prior session. When already visible,
 * a single-shot `show` is enough.
 */
/** Mirrors the `conbind_visible` atomic. The JS side reads this to decide
 * between the two-phase reveal and the single-shot `show` fallback. */
export async function isVisible(): Promise<boolean> {
  return invokeSafe<boolean>('is_visible');
}

export async function showWindow(): Promise<void> {
  const wasVisible = await isVisible();
  if (wasVisible) {
    await invokeSafe('show');
    return;
  }
  await invokeSafe('prepare_show');
  await twoFrames();
  const committed = await invokeSafe('commit_show');
  if (committed === null) {
    // prepare_show left the panel mapped at alpha 0 (or off-screen on
    // win/linux). If commit_show never runs, the launcher is invisible
    // but active. Force the single-shot reveal so the user isn't stuck.
    await invokeSafe('show');
  }
}

export async function hideWindow(): Promise<void> {
  await invokeSafe('hide');
}

export async function setFocusLock(locked: boolean): Promise<void> {
  await invokeSafe('set_focus_lock', { locked });
}

export async function quitApp(): Promise<void> {
  await invokeSafe('quit_app');
}

export async function setLauncherHeight(
  height: number,
  deferUntilNextCaCommit?: boolean,
  afterNextPresentationUpdate?: boolean,
): Promise<void> {
  await invokeSafe('set_launcher_height', {
    height,
    deferUntilNextCaCommit,
    afterNextPresentationUpdate,
  });
}

export async function setUiScale(scale: number): Promise<void> {
  await invokeSafe('set_ui_scale', { scale });
}

export async function confirmLauncherPaint(): Promise<void> {
  await invokeSafe('confirm_launcher_paint');
}

export async function cancelLauncherResize(): Promise<void> {
  await invokeSafe('cancel_launcher_resize');
}

export async function setLauncherKeepExpanded(keepExpanded: boolean): Promise<void> {
  await invokeSafe('set_launcher_keep_expanded', { keepExpanded });
}

export async function setPanelAppearance(pref: 'system' | 'light' | 'dark'): Promise<void> {
  await invokeSafe('set_panel_appearance', { pref });
}

export async function setDockIconVisible(visible: boolean): Promise<void> {
  await invokeSafe('set_dock_icon_visible', { visible });
}

export async function setTrayIconVisible(visible: boolean): Promise<void> {
  await invokeSafe('set_tray_icon_visible', { visible });
}

/**
 * Wipe everything Conbind persists, then quit. The Rust side writes a
 * sentinel into `app_data_dir` and calls `app.exit(0)`; the user must
 * manually relaunch, and the actual wipe runs at the start of that next
 * boot before any DB connection opens.
 *
 * The promise never resolves on the calling page because the process
 * exits — callers should not depend on a return value.
 */
export async function factoryReset(): Promise<void> {
  await invokeSafe('factory_reset');
}

type LauncherResetHandler = () => void;
let launcherResetHandler: LauncherResetHandler | null = null;

/**
 * Register a hook to reset launcher state when opening windows like settings.
 * Inverts the dependency so low-level IPC wrappers never import UI coordinators.
 */
export function registerLauncherResetHook(fn: LauncherResetHandler): void {
  launcherResetHandler = fn;
}

export async function showSettingsWindow(tab?: string, extensionId?: string): Promise<void> {
  await hideWindow();
  const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
  const settingsWindow = await WebviewWindow.getByLabel('settings');
  if (settingsWindow) {
    await settingsWindow.show();
    await settingsWindow.setFocus();
    if (tab) {
      const { emit } = await import('@tauri-apps/api/event');
      // Delay ensures the settings window's onMount listener is registered
      // before the event fires (relevant when the window was hidden/just shown).
      setTimeout(
        () => emit('conbind:navigate-settings-tab', { tab, extensionId: extensionId ?? null }),
        50,
      );
    }
  }
}

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WindowBoundsUpdate {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
}

export async function windowGetBounds(): Promise<WindowBounds> {
  return invokeSafe<WindowBounds>('window_management_get_bounds');
}

export async function windowSetBounds(update: WindowBoundsUpdate): Promise<void> {
  await invokeSafe('window_management_set_bounds', {
    x: update.x ?? null,
    y: update.y ?? null,
    width: update.width ?? null,
    height: update.height ?? null,
  });
}

export async function windowSetFullscreen(enable: boolean): Promise<void> {
  await invokeSafe('window_management_set_fullscreen', { enable });
}

export async function windowGetMonitors(): Promise<WindowBounds[]> {
  return invokeSafe<WindowBounds[]>('window_management_get_monitors');
}

export async function windowApplyPreset(presetId: string): Promise<void> {
  await invokeSafe('window_management_apply_preset', { presetId });
}

/** Scale the frontmost window about its center (1.1 = 10% larger). */
export async function windowResize(factor: number): Promise<void> {
  await invokeSafe('window_management_resize', { factor });
}

export async function windowMinimize(): Promise<void> {
  await invokeSafe('window_management_minimize');
}

/** Undo the last layout, resize or bounds change of the frontmost window. */
export async function windowRestore(): Promise<void> {
  await invokeSafe('window_management_restore', undefined, { silent: true });
}

export type WindowSplitAxis = 'side' | 'stack';

/**
 * Tile the frontmost window and `partnerId` side by side (or stacked) on its display and link
 * them, so resizing the shared edge of one resizes the other.
 */
export async function windowSplit(partnerId: string, axis: WindowSplitAxis): Promise<void> {
  await invokeSafe('window_management_split', { partnerId, axis });
}

/** End the linked split. Resolves `false` when none was active. */
export async function windowUnsplit(): Promise<boolean> {
  return invokeSafe<boolean>('window_management_unsplit');
}

/** One window of a workspace; `rect` holds fractions (0-1) of the display's usable area. */
export interface WorkspaceWindow {
  appId: string;
  appName: string;
  titleHint: string;
  /** Display index, ordered left to right then top to bottom. */
  monitor: number;
  rect: WindowBounds;
}

export interface CapturedWorkspace {
  windows: WorkspaceWindow[];
  /** Indices into `windows` of a pair that was linked when captured. */
  link: [number, number] | null;
}

export interface WorkspaceOpenReport {
  placed: number;
  launched: number;
  /** Apps whose window never appeared. */
  missing: string[];
  /** Window ids in workspace order; `null` where the window was not found. */
  ids: (string | null)[];
}

/** Snapshot the frontmost window and others on its display (at most 4), or just `windowIds`. */
export async function windowCaptureWorkspace(windowIds?: string[]): Promise<CapturedWorkspace> {
  return invokeSafe<CapturedWorkspace>('window_management_workspace_capture', {
    windowIds: windowIds ?? null,
  });
}

/** Arrange the windows, launching missing apps (can take several seconds). */
export async function windowOpenWorkspace(
  windows: WorkspaceWindow[],
  link: [number, number] | null,
): Promise<WorkspaceOpenReport> {
  return invokeSafe<WorkspaceOpenReport>('window_management_workspace_open', { windows, link });
}

export async function windowListWindows(): Promise<AppWindowInfo[]> {
  return invokeSafe<AppWindowInfo[]>('window_management_list_windows');
}

export async function windowFocusWindow(id: string): Promise<void> {
  await invokeSafe('window_management_focus_window', { id });
}

export async function windowCloseWindow(id: string): Promise<void> {
  await invokeSafe('window_management_close_window', { id });
}

// ── HUD ───────────────────────────────────────────────────────────────────────

export interface HudContent {
  title: string;
  spinning: boolean;
  /** Monotonic reveal generation; echo back via hudMarkShown after the
   * content has painted to complete the flash-free reveal (macOS shows
   * the HUD window at alpha 0 until then). */
  revealGen: number;
}

export async function getHudState(): Promise<HudContent | null> {
  return invokeSafe<HudContent | null>('get_hud_state');
}

/** Tells Rust the HUD content for `revealGen` is painted, so the window's
 * alpha can flip to 1. Safe to call redundantly; stale generations are
 * dropped Rust-side. No-op on non-macOS. */
export async function hudMarkShown(revealGen: number): Promise<void> {
  await invokeSafe('hud_mark_shown', { revealGen });
}

export async function showHud(args: {
  title: string;
  durationMs: number;
  spinning: boolean;
}): Promise<void> {
  await invokeSafe('show_hud', {
    title: args.title,
    durationMs: args.durationMs,
    spinning: args.spinning,
  });
}

export async function hideHud(): Promise<void> {
  await invokeSafe('hide_hud');
}

// ── Snap guides ───────────────────────────────────────────────────────────────

export interface SnapGuideState {
  leftX: number;
  rightX: number;
  y: number;
  snappedX: boolean;
  snappedY: boolean;
}

export async function getSnapGuideState(): Promise<SnapGuideState | null> {
  return invokeSafe<SnapGuideState | null>('get_snap_guide_state');
}

// ── Window dragging ───────────────────────────────────────────────────────────
// `data-tauri-drag-region` relies on the native startDragging path, which is
// not dependable for the NSPanel-converted launcher and sticky windows on
// macOS. Rust instead anchors on the window's position at pointerdown and
// applies each screen-space delta as an absolute offset — see
// `src-tauri/src/window_drag.rs`. Callers address a window by its Tauri label.

export async function windowDragStart(label: string): Promise<void> {
  await invokeSafe('window_drag_start', { label });
}

export async function windowDragMove(label: string, dx: number, dy: number): Promise<void> {
  await invokeSafe('window_drag_move', { label, dx, dy });
}

export async function windowDragEnd(label: string): Promise<void> {
  await invokeSafe('window_drag_end', { label });
}

// ── Launcher placement ────────────────────────────────────────────────────────

/** Where the launcher opens. Resolved against the target monitor on every
 * reveal, so a change takes effect on the next summon. */
export async function getLauncherPlacement(): Promise<LauncherPlacement> {
  return invokeSafe<LauncherPlacement>('get_launcher_placement');
}

export async function setLauncherPlacement(placement: LauncherPlacement): Promise<void> {
  await invokeSafe('set_launcher_placement', { placement });
}
