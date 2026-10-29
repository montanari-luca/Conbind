# Window Management

> Resize, arrange, and switch windows with layout presets and keyboard commands.

![Layout presets list](../../images/feature-window-management-hero.png)
_Figure: the layout presets list._

## What it does

Window Management lets you instantly resize, reposition, move between displays, and switch between any open windows — without touching the mouse:

- **Layout Presets**: Instantly snap the frontmost window to halves, thirds, fourths, corners, or full screen. Snapping respects the menu bar, Dock, and taskbar.
- **Resize & Minimize**: Grow or shrink the frontmost window 10% about its center, or minimize it.
- **Linked Split**: Tile two windows side by side or stacked, then drag their shared edge and the other window follows.
- **Workspaces**: Save up to four windows with their positions, then bring the whole arrangement back later, launching any app that is not running.
- **Switch Windows**: Search and jump directly to any active application window across all workspaces.
- **Multi-Display Movement**: Move the active window between external monitors with single keystrokes.
- **Custom Layouts**: Save any window's current dimensions as a reusable layout that appears in search results and can be assigned a dedicated hotkey.
- **Undo Layout Changes**: Revert accidental moves on each window with **Restore Previous Bounds**.

## How to use it

### Apply layout presets

1. Switch to the application window you want to resize.
2. Open Conbind with your global hotkey.
3. Type part of the layout name — for example `left half`, `first fourth`, `center third`, or `maximize`.
4. Press `Enter`. The window snaps immediately, a brief heads-up confirmation appears, and Conbind dismisses.

### Switch between active windows

1. Open Conbind and type `switch windows` (or `switch` or `windows`).
2. Press `Enter` to open the **Switch Windows** view.
3. A live list of all open application windows appears, showing the application icon, application name, and window title.
4. Type in the search bar to filter by app name or window title.
5. Press `Enter` on any row to focus and raise that window immediately.

### Move across multiple monitors

If you use multiple displays, use display navigation commands:

- **Next Display**: Moves the frontmost window to the next monitor while preserving its relative size.
- **Previous Display**: Moves the frontmost window to the previous monitor.

### Manage custom layouts

1. Position and resize an app window to your desired dimensions.
2. Open Conbind, search for **Save Current Window as Layout**, and press `Enter` (or open **Manage Window Layouts**, press `⌘K`, and choose **Save Current Window as Layout**).
3. Give your layout a descriptive name (e.g. "Coding Left Split").
4. Your saved layout now appears in search results alongside built-in presets.
5. To rename or delete custom layouts, open **Manage Window Layouts**, select the layout, and choose **Rename** or **Delete** from the action panel (`⌘K`).

### Resize and minimize

- **Make Window Larger** / **Make Window Smaller** grow or shrink the frontmost window by 10% about its center. The window never gets smaller than 120×80 or larger than its display's usable area, and it stays on screen.
- **Minimize Window** minimizes the frontmost window.

### Split two windows with a linked edge

1. Switch to the first window, then open Conbind.
2. Search for **Split With Window** (side by side) or **Split Stacked With Window** (top and bottom) and press `Enter`.
3. Pick the second window from the list. Both windows tile into the two halves of the first window's display.
4. Drag the shared edge of either window. The other window resizes so there are no gaps or overlaps, and neither pane gets narrower than 150 px.

The link ends by itself when you move a window away without resizing it, or close or minimize either window. Applying a layout or resize to one of the two windows also ends it. To end it manually, search for **Unlink Split**. Only one pair is linked at a time, and the link does not survive restarting Conbind.

### Save and reopen workspaces

A workspace remembers up to four windows on one display: which app, its window title, which display, and where it sat. A linked split is remembered too.

1. Arrange your windows, with the window you care most about frontmost.
2. Open Conbind, search for **Save Workspace**, and press `Enter`. The frontmost window plus the next frontmost windows on the same display (up to four) are saved as **Workspace 1**, **Workspace 2**, and so on.
3. Open **Manage Workspaces** to rename, update, or delete workspaces. Select one and press `⌘K` for **Open**, **Update with Current Windows**, **Rename**, or **Delete**.
4. To bring a workspace back, search for its name (saved workspaces appear in search results), or open **Manage Workspaces** and press `Enter`. You can also assign a hotkey to any workspace.

When you open a workspace, Conbind reuses windows that are already running, matching by app and then by window title. Apps that are not running are launched, and Conbind waits up to about 10 seconds for each window to appear. The windows are placed (positions are stored relative to the display, so they adapt to a different resolution), raised so the first one ends up on top, and a saved split is linked again. If a window never shows up, the confirmation names the missing app and the rest are still arranged.

The **Open Workspace** and **Update Workspace** commands also accept a workspace `name` or `id` when run from a script or hotkey. **Update Workspace** with no name overwrites the workspace you opened or saved last.

### Undo the last layout change

- Type `restore` in the search bar and press `Enter`, or search for **Restore Previous Bounds**.
- Undo is remembered separately for each window: restoring window A does not affect what window B will restore to.

## Shortcuts & actions

| Action                   | How                                           |
| ------------------------ | --------------------------------------------- |
| Apply any preset         | Search its name, then `Enter`                 |
| Switch active windows    | Search `switch windows`, filter, then `Enter` |
| Move to next display     | Search `next display`, then `Enter`           |
| Move to previous display | Search `previous display`, then `Enter`       |
| Undo last layout         | Search `restore`, then `Enter`                |
| Grow / shrink window     | Search `larger` or `smaller`, then `Enter`    |
| Minimize window          | Search `minimize`, then `Enter`               |
| Link two windows         | Search `split with window`, pick the partner  |
| End a linked split       | Search `unlink split`, then `Enter`           |
| Save a workspace         | Search `save workspace`, then `Enter`         |
| Open a workspace         | Search its name, then `Enter`                 |
| Open action panel        | `⌘K` (in the Manage and Switch Windows views) |

**Action panel (⌘K) entries in Manage Layouts:**

- **Save Current Window as Layout** — captures the active window's current bounds.
- **Rename** — renames the selected custom layout.
- **Delete** — removes the selected custom layout.

## Built-in presets

| Preset                        | What it does                                   |
| ----------------------------- | ---------------------------------------------- |
| **Left Half**                 | Left 50% of the screen                         |
| **Right Half**                | Right 50% of the screen                        |
| **Top Half**                  | Top 50% of the screen                          |
| **Bottom Half**               | Bottom 50% of the screen                       |
| **Top Left Quarter**          | Top-left 25% corner                            |
| **Top Right Quarter**         | Top-right 25% corner                           |
| **Bottom Left Quarter**       | Bottom-left 25% corner                         |
| **Bottom Right Quarter**      | Bottom-right 25% corner                        |
| **Left Third**                | Left 33% column                                |
| **Center Third**              | Center 33% column                              |
| **Right Third**               | Right 33% column                               |
| **Left Two Thirds**           | Left 66% width                                 |
| **Right Two Thirds**          | Right 66% width                                |
| **First Fourth**              | First 25% vertical column (leftmost)           |
| **Second Fourth**             | Second 25% vertical column (center-left)       |
| **Third Fourth**              | Third 25% vertical column (center-right)       |
| **Last Fourth**               | Fourth 25% vertical column (rightmost)         |
| **Center**                    | Centered window (80% width × 80% height)       |
| **Almost Maximize**           | Centered window (90% width × 90% height)       |
| **Maximize**                  | Full screen                                    |
| **Next Display**              | Move to next monitor                           |
| **Previous Display**          | Move to previous monitor                       |
| **Make Window Larger**        | Grow the window 10% about its center           |
| **Make Window Smaller**       | Shrink the window 10% about its center         |
| **Minimize Window**           | Minimize the window                            |
| **Split With Window**         | Tile two windows side by side, linked          |
| **Split Stacked With Window** | Tile two windows top and bottom, linked        |
| **Unlink Split**              | Stop resizing the linked pair together         |
| **Save Workspace**            | Save up to 4 windows and their positions       |
| **Open Workspace**            | Arrange a saved workspace                      |
| **Update Workspace**          | Overwrite a workspace with the current windows |
| **Manage Workspaces**         | Open, update, rename, or delete workspaces     |
| **Restore Previous Bounds**   | Undo the last layout change for a window       |

## Tips

- **Assign hotkeys for common splits**: Open **Settings → Extensions**, expand **Window Management**, and bind hotkeys like `⌃⌥←` for Left Half, `⌃⌥→` for Right Half, and `⌃⌥Enter` for Maximize.
- **Switch app first, then open Conbind**: Layout presets apply to whichever window was active immediately before summoning Conbind.
- **Vertical Fourths on ultrawide monitors**: The `First Fourth` through `Last Fourth` presets are ideal for 34"+ ultrawide and 4K displays where half-screen windows are too wide.
- **Restore only remembers one step per window**: Restore undoes the most recent layout, resize, or split placement of that window, and it does not survive restarting Conbind.
- **Workspaces are per display**: A workspace captures the display the frontmost window is on. Windows on other displays are not included.
- **Disabling Window Management** — If you use a third-party tiling window manager or prefer not to have window snap commands in search, you can turn off Window Management in **Settings → Extensions**. Disabling removes preset commands and layouts, while the underlying window management platform APIs (`window:manage`) remain available for extensions.

## Platform notes

- **macOS** needs Accessibility permission (**System Settings → Privacy & Security → Accessibility**). Only windows on the current Space are visible to window management.
- **Windows** positions windows by their visible edges, so snapped windows sit flush with no gap from the invisible resize border.
- **Linux** works in an X11 session with a window manager that supports the standard EWMH hints (GNOME, KDE, Xfce, i3, and most others). Wayland sessions are not supported. Minimizing, snapping, and the usable area (excluding panels and docks) follow what your window manager reports, so small differences between desktops are possible.
- Linked split, workspaces, resize, minimize, and per-window restore are new and have been unit-tested against a simulated window backend. The live macOS, Windows, and Linux window calls behind them have not yet been tried on real machines.

## Related

- [The Basics](../the-basics.md)
- [Aliases & Shortcuts](./aliases-and-shortcuts.md)
- [Keyboard Shortcuts](../keyboard-shortcuts.md)
- [Walkthrough](./walkthrough.md)
