# Resource Usage

> A live view of your CPU, GPU, RAM, VRAM, network, and disk usage, one command away.

## What it does

**View resource usage** opens a dashboard that updates every second while it is open. It reads your system's own counters, makes no network calls, and keeps no history.

| Card        | What it shows                                                                         |
| ----------- | ------------------------------------------------------------------------------------- |
| **CPU**     | Overall load and the number of cores                                                  |
| **RAM**     | Memory in use, with swap when your system has it                                      |
| **GPU**     | How busy the graphics processor is                                                    |
| **VRAM**    | Video memory in use. On Apple Silicon this is the share of shared memory the GPU uses |
| **Network** | Current download and upload speed, plus totals since boot                             |
| **Disk**    | Used and total space for each volume                                                  |

## How to use it

1. Open Conbind and type `resource usage` (or `cpu`, `ram`, `gpu`, `disk`, `network`, …).
2. Press `Enter` on **View resource usage**.
3. Press `Esc` to go back. Sampling stops as soon as you leave the view.

## Tips

- **"Not available" is normal for some GPUs.** GPU and VRAM figures come from whatever your platform exposes. Cards without a readable counter show their name or "Not available" instead of a number.
- **Disk shows space, not speed.** Each volume reports how full it is. Read and write throughput is not shown.
- **Network ignores loopback.** Traffic to your own machine is not counted.

## Platform notes

| Platform | GPU and VRAM source                                                         |
| -------- | --------------------------------------------------------------------------- |
| macOS    | The system's graphics accelerator statistics (Apple Silicon and AMD)        |
| Linux    | AMD and Intel counters under `/sys/class/drm`, plus `nvidia-smi` if present |
| Windows  | `nvidia-smi`, if present. Other GPUs show "Not available".                  |

CPU, RAM, network, and disk work the same on every platform. Developed and tested on **macOS**; the Windows and Linux GPU paths are unit-tested but have not been tried on real machines yet.

## Related

- [Cleanup](./cleanup.md) — free up disk space when the Disk card shows a volume filling up.
