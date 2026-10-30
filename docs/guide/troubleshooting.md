# Troubleshooting

> Fixes for the most common problems.

## The hotkey doesn't open Conbind

If pressing your global hotkey does nothing:

1. **Check that Conbind is running.** Look for the Conbind icon in the menu bar. If it is not there, open Conbind from your Applications folder.

2. **Verify the hotkey is set.** Open **Settings → Shortcuts** (or **Settings → General → Hotkey**). The recorder should show your current combination. If it is blank, click inside and press your keys again, then save.

3. **Check for conflicts.** Another app may have claimed the same key combination. Open the shortcut recorder — if a conflict warning appears, choose a different combination that is not already in use.

4. **Linux Wayland Shortcut Setup:** On Wayland (GNOME, KDE, Hyprland, Sway), applications cannot intercept global keystrokes in the background. You must create a custom shortcut in your desktop settings that executes `conbind`:
   - **GNOME:** **Settings** → **Keyboard** → **View and Customize Shortcuts** → **Custom Shortcuts** → Add Name `Conbind`, Command `conbind`, and set your shortcut (e.g. `Ctrl+Space`).
   - **KDE Plasma:** **System Settings** → **Shortcuts** → **Custom Shortcuts** / **Command/URL** → set Command to `conbind`.
   - **Hyprland:** Add `bind = ALT, SPACE, exec, conbind` to `hyprland.conf`.
   - **Sway:** Add `bindsym Mod1+space exec conbind` to your Sway config.
     Running `conbind` while the app is active will toggle the launcher window.

5. **Check macOS Accessibility permission (macOS only).** Although Accessibility is mainly needed for text expansion, Conbind also needs it for reliable global hotkey registration on some macOS versions. Go to **System Settings → Privacy & Security → Accessibility**, find Conbind in the list, and make sure the toggle is on. Relaunch Conbind after granting access.

6. **Restart Conbind.** Search for "Quit Conbind" inside the launcher (if you can open it another way), or quit via the menu bar / system tray icon and reopen Conbind.

7. **Restart your computer.** Occasionally a fresh login is needed after granting system permissions for the first time.

## Conbind can't see my apps / accessibility

**Apps are missing from search:**

- Conbind scans `/Applications` and `~/Applications` by default. If your apps live somewhere else, go to **Settings → Applications** and click **Add Directory** to add the folder.
- Check that the app's toggle is on in the app list. If you previously disabled an app, its toggle is off.
- Wait a moment after installing a new app — Conbind watches the scan directories and picks up changes automatically, but it can take a few seconds.

**Accessibility permission not granted (macOS only):**

> **Windows and Linux users:** Accessibility permission is a macOS-only requirement. If you are on Windows or Linux and text expansion is not working, skip this section and check that **Settings → Advanced → Text Expansion** is enabled.

On macOS, Conbind needs Accessibility access to paste text snippets, capture selected text, move or resize other apps' windows and, when the system media helper is unavailable, send media keys. Without it, text expansion, selection-aware AI commands and window management commands (Center, Left Half and so on) will not work, and Play / Pause, Next Track and Previous Track fall back to Music and Spotify only. The first time you run a window command without the permission, macOS shows a prompt that adds Conbind to the list and offers to open System Settings.

1. Open **System Settings → Privacy & Security → Accessibility**.
2. Find Conbind in the list. If it is not there, drag the Conbind app from your Applications folder into the list.
3. Enable the toggle next to Conbind.
4. If Conbind was already in the list but toggled off, toggle it off and back on, then relaunch Conbind.

You can also reach this screen directly from **Settings → General → Re-run onboarding**, which guides you through the accessibility step again.

## Search feels slow

Conbind's search is designed to be instant. If it feels slow:

- **First launch is slower.** On the very first launch Conbind indexes your applications and files. This runs in the background and takes only a few seconds. Subsequent searches are fast.

- **Too many scan directories.** If you added a very large directory (for example, your entire home folder) in **Settings → Applications**, Conbind may be scanning many files. Remove overly broad directories and add only the specific folders you need.

- **Restart Conbind.** Quit using the "Quit Conbind" command and reopen from Applications.

- **Restart your computer.** A full restart can resolve issues where background indexing stalled.

## AI isn't responding

If the AI chip is missing or AI queries return no response:

1. **Check that a provider is configured.** Open **Settings → AI**. If the providers list is empty, click **+ Add provider**, choose a provider (for example, Anthropic or OpenAI), and enter your API key.

2. **Check your API key.** An incorrect or expired API key causes silent failures. In the provider row, expand it and re-enter the key, then use the Test connection option if available.

3. **Check your network.** AI responses require an internet connection to the provider's API. Make sure your computer is online and not behind a firewall that blocks the provider's domain.

4. **Check the default provider.** In **Settings → AI**, confirm that the star (★) is set next to the provider you want to use. The starred provider powers the `Tab` AI chip.

5. **Check the model selection.** If you recently changed your model and the new model is unavailable on your plan, responses may fail silently. Click the provider row to expand it and pick a model you have access to.

## Browser bookmarks/history aren't showing

Conbind can search your browser's open tabs, bookmarks, and history, but it requires the Conbind companion extension to be installed in your browser and paired.

1. **Install the companion extension.** Download the Conbind companion for your browser (Chromium-based browsers are currently supported). Install it from your browser's extension page.

2. **Pair the browser.** After installation, the companion sends a pairing request to Conbind. Open **Settings → Browsers** — pending pairing requests appear there. Click **Allow** to approve the connection.

3. **Check the connection status.** Once paired, the browser appears in the paired list with a connection indicator. If it shows as disconnected, try restarting the browser with the companion extension active.

4. **Check that browser search is enabled.** In **Settings → Advanced**, confirm that Extension Search is turned on. Without it, browser results are not shown in the main search.

5. **Re-pair if needed.** If the companion and Conbind lose their connection after an update, revoke the old pairing in **Settings → Browsers → (remove)** and pair again.

## Raycast import skipped some hotkeys or aliases

This is expected in a few specific cases — Conbind tells you the count but not always the reason:

- **Bound to a Raycast command or extension.** Hotkeys and aliases on Raycast's own built-in commands (Clipboard History, Dictionary, File Search, and similar) or on third-party Raycast extensions have no Conbind equivalent, so they cannot be imported.
- **The app isn't installed here.** An app hotkey or alias only imports if Conbind finds that exact application on this computer. If you're setting up a new machine, install the app first, then re-run the import.
- **The alias has unsupported characters.** Conbind aliases must be 1–10 lowercase letters or digits (see [Aliases & Shortcuts](./features/aliases-and-shortcuts.md)). A Raycast alias with symbols, spaces, or uppercase letters is skipped rather than imported broken.

See [Import from Raycast](./features/raycast-import.md) for the full picture of what does and doesn't carry over.

## Raycast import says my password is wrong

- **Use the export password, not your Raycast account password.** This is the password you set (or left blank) in Raycast's **Export Settings & Data** dialog, specific to that one export file.
- **Re-export if you're not sure.** If you no longer remember the export password, go back to Raycast and run **Export Settings & Data** again with a new password (or no password).

## Related

- [Getting Started](./getting-started.md)
- [Settings](./settings.md)
- [Import from Raycast](./features/raycast-import.md)
- [FAQ](./faq.md)
