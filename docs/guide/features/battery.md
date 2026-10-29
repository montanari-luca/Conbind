# Battery

> Ask how much longer your laptop battery will last, one command away.

## What it does

**Estimate battery** opens a small view with your charge level and the time your system says is left. It uses the estimate your operating system already calculates, makes no network calls, and keeps no history.

| Situation                | What you see                                               |
| ------------------------ | ---------------------------------------------------------- |
| On battery               | Time left (for example "3 h 12 min left") and charge       |
| Charging                 | Time until full, or "Charging" if there is no estimate yet |
| Fully charged            | "Fully charged"                                            |
| Just unplugged           | "Calculating…" until your system has an estimate           |
| No battery (desktop, VM) | "No battery detected"                                      |

## How to use it

1. Open Conbind and type `battery` (or `estimate battery`, `battery left`, …).
2. Press `Enter` on **Estimate battery**.
3. Press `Esc` to go back. The view re-reads every few seconds while it is open and stops as soon as you leave.

## Tips

- **The estimate moves.** It is based on what you are doing right now, so heavy work shortens it and idling lengthens it.
- **"Calculating…" is normal** for a minute or so after unplugging. Systems need a little usage data first.

## Platform notes

| Platform | Source                                 |
| -------- | -------------------------------------- |
| macOS    | IOKit power sources                    |
| Linux    | UPower over DBus (the display battery) |
| Windows  | `GetSystemPowerStatus`                 |

Developed and tested on **macOS**; the Windows and Linux paths are unit-tested but have not been tried on real machines yet. Windows does not report a time-to-full, so charging shows "Charging".

## Related

- [Resource Usage](./resource-usage.md) — Live CPU, GPU, RAM, VRAM, network, and disk usage.
