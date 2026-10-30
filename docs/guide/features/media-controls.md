# Media Controls

> Play, pause and skip tracks from the launcher, without switching to the player.

## What it does

Media Controls sends play, pause, next and previous to whatever your system considers the active player, and shows what is playing right now. It works with music apps and with video playing in a browser tab. Everything stays on your device; nothing is recorded or sent anywhere.

| Command            | What it does                         |
| ------------------ | ------------------------------------ |
| **Play / Pause**   | Toggles playback                     |
| **Next Track**     | Skips to the next track              |
| **Previous Track** | Goes back to the previous track      |
| **Stop Playback**  | Stops the player (pauses on Spotify) |

A short message confirms what was sent, then the launcher closes.

## Now Playing

Type `now playing`, `music` or `media` and a row appears with the current track, its artist and the app playing it. Nothing shows when nothing is playing.

- Press `Enter` to play or pause.
- Open the actions menu (`⌘K` / `Ctrl+K`) for **Next track** and **Previous track**.

## How to use it

1. Open Conbind and type `pause`, `next`, `previous` or `stop`.
2. Press `Enter` on the command.

To turn the feature off, use **Settings → Extensions → Media Controls**.

## Tips

- **Assign a hotkey.** Give each command a global shortcut in [Aliases & Shortcuts](./aliases-and-shortcuts.md) to skip tracks without opening the launcher.
- **No player running?** The command stays open and tells you nothing accepted it, rather than starting an app.

## Platform notes

| Platform | Source                                                                   |
| -------- | ------------------------------------------------------------------------ |
| macOS    | System-wide media state (see below), then media keys, then Music/Spotify |
| Linux    | MPRIS over the session bus (the standard for Linux media players)        |
| Windows  | System Media Transport Controls (the same source as the volume flyout)   |

**macOS** uses a bundled helper because Apple restricted the system media API in macOS 15.4. If the helper does not work on your macOS version, Conbind falls back automatically:

1. **Media keys** for play/pause, next and previous. They work with any player but need Accessibility permission; macOS asks the first time. See [Troubleshooting](../troubleshooting.md).
2. **Music and Spotify** through AppleScript for the Now Playing row and Stop, and for the other commands if media keys are not allowed. macOS asks once per app for permission to control it.

Developed and tested on **macOS**. The Linux and Windows paths are implemented but have not been tried on real machines yet.

## Related

- [System Commands](./system.md) — Sleep, Lock Screen, Restart, Shut Down and Log Out.
- [Aliases & Shortcuts](./aliases-and-shortcuts.md) — Global hotkeys for any command.
