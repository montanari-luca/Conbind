### 8.24 `WindowManagementService` — Frontmost window bounds and fullscreen

**Runs in:** view only. Window management is invoked from a user-driven
view action.

**Permission required:** `window:manage`

The `WindowManagementService` lets extensions read and manipulate the position, size, and fullscreen state of the **frontmost OS application window** — the window the user had open before invoking Conbind.

```typescript
interface IWindowManagementService {
  /** Returns the current bounds of the frontmost OS application window. */
  getWindowBounds(): Promise<WindowBounds>;

  /**
   * Updates the position and/or size of the frontmost OS application window.
   * Only the fields you provide are changed; omit the rest.
   */
  setWindowBounds(update: Partial<WindowBounds>): Promise<void>;

  /**
   * Toggles the fullscreen state of the frontmost OS application window.
   * On Windows this maps to maximize / restore.
   */
  setFullscreen(enable: boolean): Promise<void>;

  /**
   * Returns the usable area of every display: its bounds minus the menu bar, Dock,
   * taskbar, and panels.
   */
  getMonitors(): Promise<WindowBounds[]>;

  /** Applies a preset layout to the frontmost OS application window. */
  applyPreset(presetId: string): Promise<void>;

  /** Moves frontmost window to the previous display, keeping relative size and position. */
  previousDisplay(): Promise<void>;

  /** Moves frontmost window to the next display, keeping relative size and position. */
  nextDisplay(): Promise<void>;
}

interface WindowBounds {
  /** Pixels from the left edge of the desktop (screen points on macOS, physical pixels elsewhere). */
  x: number;
  /** Pixels from the top edge of the desktop. */
  y: number;
  /** Width in pixels. */
  width: number;
  /** Height in pixels. */
  height: number;
}
```

**Usage:**

```typescript
const winService = context.getService<IWindowManagementService>('window');

// Read current bounds
const bounds = await winService.getWindowBounds();
console.log(`Window at (${bounds.x}, ${bounds.y}), size ${bounds.width}×${bounds.height}`);

// Move window right by 50px without changing size
await winService.setWindowBounds({ x: bounds.x + 50, y: bounds.y });

// Resize to 1280×800 without moving
await winService.setWindowBounds({ width: 1280, height: 800 });

// Enter fullscreen
await winService.setFullscreen(true);
```

### Platform Notes

#### macOS

Uses the **Accessibility API** (`AXUIElement`) to read and write the focused window's `AXPosition`, `AXSize`, and `AXFullScreen` attributes. This requires **Accessibility permission** granted to Conbind in **System Settings → Privacy & Security → Accessibility**. If the permission is missing, all three methods throw with a descriptive error pointing to the settings panel.

`getMonitors` excludes the menu bar and Dock. Coordinates use a top-left origin and may be negative for windows on a monitor to the left of the primary display — this is correct behaviour.

#### Windows

Targets the window that was foreground when Conbind was last opened (captured via `GetForegroundWindow` at show time). `setWindowBounds` places the window with `SetWindowPos` so that its _visible_ edges (the DWM frame, not the invisible resize border) land on the bounds you give; all coordinates are physical pixels, matching `getMonitors`; `setFullscreen(true)` maximizes the window and `setFullscreen(false)` restores it. True borderless fullscreen for external windows is not supported.

#### Linux (X11)

Talks to the window manager directly over X11 (EWMH), so nothing extra needs to be installed. Conbind captures the active window ID from `_NET_ACTIVE_WINDOW` before showing itself. Coordinates are physical pixels, and `getMonitors` excludes space reserved by panels and docks (`_NET_WM_STRUT_PARTIAL`). Your window manager must support EWMH.

**Wayland is not supported.** Calling any method under a Wayland session returns an error with a message directing the user to use an X11/XOrg session.

---
