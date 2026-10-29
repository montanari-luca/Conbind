# Cleanup

> Free up disk space from the launcher: caches, developer leftovers, app remnants, a space analyzer, and network cache cleanup.

## What it does

Cleanup finds things you can safely get rid of and moves them to the **Trash**, so every cleanup can be undone until you empty it. It only ever touches files inside your home folder, and nothing is deleted until you review the list and confirm.

| Command                 | What it finds                                                                                                                                            |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Clean Up**            | App caches, logs, and temporary files                                                                                                                    |
| **Developer Cleanup**   | Xcode DerivedData and device support files, plus npm, pnpm, Cargo, and Gradle caches                                                                     |
| **Find App Leftovers**  | Data left behind by apps you've uninstalled (macOS). Nothing is pre-selected: each item is opt-in, because this one is a best guess.                     |
| **Disk Space Analyzer** | A drill-down view of your home folder showing which folders and files use the most space                                                                 |
| **Network Cleanup**     | Flushes the DNS cache (and the ARP table on macOS). Your system asks for your administrator password; Conbind never sees it.                             |
| **Empty Trash**         | Permanently deletes everything in the Trash. This is the only permanent action, and it asks first. Disk space only comes back once the Trash is emptied. |

## How to use it

1. Open Conbind and type `clean up` (or `developer cleanup`, `disk space`, …).
2. Sizes fill in as each category finishes scanning. Expand a category to see the individual items.
3. Untick anything you want to keep.
4. Press `Enter` (or click **Move to Trash**) and confirm.

In the **Disk Space Analyzer**, move through folders with the arrow keys, press `Enter` to open one, and `←` or `Backspace` to go back up. Open the action panel (`⌘K`) to reveal an item in your file manager or move it to the Trash.

## Shortcuts & actions

| Action                       | How                |
| ---------------------------- | ------------------ |
| Move selection               | `↑` / `↓`          |
| Expand / collapse a category | `→` / `←`          |
| Tick / untick                | `Space`            |
| Move ticked items to Trash   | `Enter`            |
| Analyzer: open folder        | `Enter` or `→`     |
| Analyzer: go up              | `←` or `Backspace` |
| Open action panel            | `⌘K`               |

## Tips

- **Full Disk Access (macOS).** Without it, macOS hides some folders from Conbind and the analyzer can under-count `~/Library`. Cleanup shows a banner with a shortcut to the right System Settings pane when it detects this.
- **Leftovers are a heuristic.** Conbind only suggests data named after an app's bundle id that matches no installed app, ignores Apple's and shared data, and skips anything modified in the last week. Apps that live outside `/Applications` can still be flagged, so look before you tick.
- **Nothing leaves your device.** Cleanup makes no network calls and keeps no history.

## Platform notes

Cleanup is developed and tested on **macOS**. The scan targets, space analyzer, and Trash handling are cross-platform and unit-tested, but the Windows and Linux paths (including Network Cleanup and Empty Trash) have not been tried on real machines yet. **Find App Leftovers** is macOS-only.

## Related

- [File Search](./file-search.md) — find individual files by name.
- [Clipboard History](./clipboard-history.md) — another place Conbind keeps local data, with its own retention settings.
