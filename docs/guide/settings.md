# Settings

> A tour of every settings page.

![The General settings tab](../images/settings-general.png)
_Figure: the General settings tab._

Open Settings with `⌘,` from anywhere inside the launcher, or search for "Settings" and press `Enter`. A sidebar on the left groups the pages: **Basics** (General, Appearance), **Search** (Extensions, Applications, Browsers, File Search, Scripts), **Data & AI** (AI, Backup, Privacy) and **System** (Advanced, About, plus Developer while developer mode is on).

### Searching settings

The search box at the top of the sidebar finds individual settings across every page. Press `⌘F` (`Ctrl+F` on Windows and Linux) or `/` to jump to it, type a few words, use `↑`/`↓` and `Enter` (or click) to open a result, and `Esc` to clear. Selecting a result opens its page, scrolls to the setting and briefly highlights it.

## General

The General page covers startup and system integration:

- **Startup** — Toggle "Launch Conbind at login" to have Conbind start automatically when you log in to your computer.
- **Hotkey** — Change the global show/hide shortcut. Click inside the recorder, press your new combination (including standalone `F1`–`F24` keys or Hyper key combinations), and click Save.
- **Menu Bar Icon** — Toggle "Show icon in menu bar" to control whether Conbind appears in the system menu bar / system tray.
- **Dock Icon** (macOS) — Toggle "Show icon in Dock" to control whether Conbind appears in the macOS Dock and ⌘Tab application switcher.
- **Onboarding** — Click "Re-run onboarding" to walk through the welcome flow again — useful if you skipped any steps the first time.

## Appearance

The Appearance page controls how Conbind looks and where the launcher appears:

- **Theme** — Switch between Light, Dark, or System (follows your operating system).
- **Color palette** — Choose **Blue Mint** (the default: ocean-blue surfaces with a mint accent) or **Classic** (the neutral grey-and-blue look). Both have light and dark variants and follow the Theme setting.
- **Window Mode** — Choose between Default (search bar + results list visible immediately) and Compact (search bar only; results expand when you type).
- **Interface Size** — Scale the launcher's text and controls.
- **Placement & Drag to Snap** — Choose which display the launcher opens on, its vertical position, and whether alignment snap guides appear when you reposition it. Placement is stored locally per machine so multi-monitor differences don't clash between machines.
- **Custom Themes** — If you have installed community themes, a theme picker appears here alongside the built-in Default. Select a theme to apply it instantly.

## Shortcuts

The Shortcuts tab has one purpose: changing the global activation shortcut.

- **Conbind activation shortcut** — Click inside the recorder and press the shortcut you want. Conbind supports standard modifier combinations (e.g. `⌥Space`), Hyper key combinations (all four modifiers, rendered as `✦`), and standalone function keys (`F1`–`F24`, including extended keys `F13`–`F24` and the Windows Copilot key `F23`) without modifiers for dedicated macro keys or remapped `Caps Lock`. Conbind checks for conflicts with other registered shortcuts and warns you if the combination is already in use. Click Save to apply it.

All other per-item shortcuts (for commands, snippets, scripts, and agents) are managed from the **Applications & Extensions** tab and from each item's row in the launcher.

## Applications & Extensions

This tab has two sub-sections that share a tab.

**Applications** lets you control which apps Conbind indexes:

- **Scan directories** — Conbind scans your Applications folder by default. Click "Add Directory" to add more folders (for example, a custom tools folder). Default folders are shown as read-only; custom folders can be removed.
- **Per-app toggles** — A list of every discovered app. You can hide individual apps from search results by turning off their toggle. You can also assign a keyboard shortcut or a search alias to any app from its row.

**Extensions** (the Extensions tab) manages both built-in features and installed extensions:

- **Built-in Features vs. Installed Extensions** — The list is organized into distinct sections with item counts for built-in capabilities (like Calculator, Snippets, Screen OCR, Window Management, and Clipboard) and installed third-party extensions.
- **Filter Chips** — Scope the list using filter chips at the top:
  - **All** — Shows all built-in features and installed extensions.
  - **Commands** — Shows only extensions and features that declare runnable commands.
  - **Extensions** — Scopes specifically to third-party installed extensions (excluding built-in features).
  - **Themes** — Shows only installed theme extensions.
- **Toggles & Disableability** — Turn any extension or optional built-in feature on or off with its toggle. All 21 optional built-in features (such as Calculator, File Search, Snippets, Screen OCR, Notes, Runs, Window Management, etc.) can be turned off to streamline your launcher UI; when disabled, their commands, views, and search entries are removed, while underlying platform services remain running and accessible to Tier 2 extensions. Essential platform infrastructure (`System` and `Settings`) displays a locked, non-interactive toggle with a tooltip explaining that core infrastructure cannot be disabled. Expand an extension with the chevron to view its individual commands and directly assign search aliases or global hotkeys.
- **Detail Panel** — Selecting any feature, extension, or command opens the right-hand panel displaying its icon, metadata badges (`Built-in`, `Extension`, `Theme`, `Command`), description, trigger chip, permissions, runtime status, and configuration preferences.
- **Local Network Devices** — For extensions that request the `network:lan` permission, the detail panel includes a **Local network devices** section where you can view approved endpoints, run a local scan to discover and approve devices (Sonos, Google Cast, Home Assistant, etc.), add custom IP/port endpoints manually, or revoke existing approvals.
- **Install & Update** — Use the **+** button in the toolbar to create or install extensions from a file (developer mode), and update available extensions directly from this view. Auto-update behaviour is controlled in Advanced.

## File Search

- **Enable/disable** — Turn the whole feature off if you don't want your files indexed. Turning it back on rebuilds the index.
- **Show in Root Search** — Toggle whether matching files and folders appear directly in the main launcher search results (up to 4 inline matches) or stay exclusive to the dedicated File Search view.
- **Index Status** — Shows whether the index is Building, Ready, Rescanning, or has hit its size cap, plus how many files are indexed and whether it was restored from a snapshot on this launch. Click **Rebuild Index** to force a fresh scan.
- **Search Roots** — Empty means your entire home folder. Click **Add Root** to narrow the scope to specific directories instead.
- **Exclude Patterns** — Add glob patterns (like `*.tmp`) on top of the built-in exclusions (caches, `node_modules`, `.git`, VM disk images, and other system directories).

See [File Search](./features/file-search.md) for how to use the feature itself.

## AI, MCP & Browsers

This section is split across two related tabs.

**AI** tab:

- **Tab continues last thread** — When on, pressing `Tab` to enter AI mode resumes your previous conversation instead of starting a new one.
- **History retention cap** — Maximum conversation sessions to keep before older unpinned threads are automatically deleted (Unlimited, 25, 50, 100 [default], or custom). Pinned threads are always preserved.
- **Providers** — Add one or more AI providers (Anthropic, OpenAI, OpenRouter, Google, Ollama, or a custom endpoint).
- **Local CLI Providers (Zero-Key)** — Connect directly to your local authenticated Google (`agy`) and OpenAI (`codex`) CLIs without needing to enter an API key.
- **Searchable Model Filter** — Easily search and filter through large provider catalogs (e.g. OpenRouter or Ollama).
- **Web Search & Grounding** — Configure web search capabilities for your agents:
  - **Default engine** — Choose between DuckDuckGo (free, unauthenticated fallback), Brave Search, or Tavily.
  - **API Keys** — Enter Brave or Tavily API keys for higher volume queries.
  - **Google Search Grounding** — Enable native Google Search grounding for Gemini models.
- **Advanced AI settings** — Max tokens, temperature, and reasoning controls.
- **Manage Agents** — Open the Agents view to create, edit, and assign hotkeys to AI agents.
- **MCP** — Connect Model Context Protocol servers to give your AI agents access to external tools and data.

**Browsers** tab:

- **Connected browsers** — Shows which browsers have the Conbind companion extension installed and paired. A paired browser shares its open tabs, bookmarks, and history with Conbind's search.
- **Pending pairings** — When a browser requests pairing, a prompt appears here. Allow or deny each request.
- **Paired browser list** — Revoke a pairing at any time by clicking Remove next to the browser name.

## Privacy, Scripts & Advanced

**Privacy** tab:

- **Encryption status** — Shows whether Conbind's local data store is encrypted on disk.
- **Clipboard privacy** — Configure the history retention period (TTL from 1 minute up to 90 days), inspect session capture stats, and manage application denylist rules to prevent sensitive apps (like password managers) from being captured into Clipboard History.
- **Secret redaction** — Automatically strip API keys and tokens from clipboard entries before they are stored.

**Scripts** tab:

- **Script directories** — Add folder paths that Conbind should watch for executable scripts. Any script placed in a watched folder is discoverable from the launcher immediately — no restart required. Remove a folder to stop watching it.

**Advanced** tab:

- **Extension Search** — Toggle whether installed extensions can contribute results to the main search bar.
- **Escape Key** — Choose how `Esc` behaves when a view is open: Step Backwards (default), Hide Window, or Reset Launcher.
- **Auto Updates** — Extensions update silently in the background when this is on.
- **Text Expansion** — Enable or disable snippet text expansion. Requires Accessibility permission on macOS (macOS only).
- **Developer Mode** — Unlocks the extension inspector, verbose logging, and extension sideloading. Intended for extension developers.

## Backup & About

**Backup** tab:

- **Export** — Select which data categories to include, optionally set a password, and click Export to save a local backup file.
- **Import** — Choose a backup file to restore from. You can pick which categories to restore and how to handle conflicts (Merge, Replace, or Skip).
- **Migrate from Raycast** — Click **Import from Raycast…** to bring in snippets, quicklinks, and app hotkeys from Raycast exports (both compressed and uncompressed JSON formats supported). See [Import from Raycast](./features/raycast-import.md).

**About** tab:

- **Version** — Shows the current Conbind version.
- **Update channel** — Switch between Stable and Beta update channels.
- **Check for updates** — Manually trigger an update check. If a newer version is available, a **Download** button opens the installer link for your platform; run it to update.

## Related

- [Backup](./backup.md)
- [Import from Raycast](./features/raycast-import.md)
- [Getting Started](./getting-started.md)
- [Keyboard Shortcuts](./keyboard-shortcuts.md)
- [Troubleshooting](./troubleshooting.md)
- [Screen OCR](./features/screen-ocr.md)
- [System Commands](./features/system.md)
- [Walkthrough](./features/walkthrough.md)
- [Usage Statistics](./features/usage-stats.md)
