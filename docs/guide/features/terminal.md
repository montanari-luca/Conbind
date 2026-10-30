# Terminal

> Run a shell command without leaving the launcher: start your search with `>`.

## What it does

Type `>` followed by a command and Conbind runs it in your shell and streams the output into a view inside the launcher. The folder you are in and the environment variables you set carry over from one command to the next until you quit Conbind. Nothing is sent anywhere and nothing is written to disk besides what your commands do themselves.

## How to use it

1. Open Conbind and type `>` and a command, for example `> ls -la` or `>git status`.
2. Press `Enter` on **Run in the terminal**. The view opens and the command runs.
3. Type further commands in the input at the bottom and press `Enter`. `↑` and `↓` walk through earlier commands.
4. Press `Ctrl+C` to stop a command that is still running, and `Esc` to go back. The scrollback is kept until you quit Conbind. Type `clear` to empty it.

You can also type `terminal` in the launcher to open the view without a command.

## What carries over

| Carries over                             | Does not carry over                    |
| ---------------------------------------- | -------------------------------------- |
| Folder (`cd`)                            | Aliases and shell functions            |
| Environment (`export`, `unset`, `$PATH`) | Shell options and history of the shell |

Each command runs in a fresh, non-interactive shell, so the first command goes through a login shell to pick up your usual `PATH`, and the resulting folder and environment are handed to the next one. Interactive programs (`vim`, `top`, `ssh` prompts) are not supported because there is no real terminal behind the view.

## Tips

- **A failing command shows its exit code** under its output.
- **Output is capped** at 5000 lines per command and 100 commands in the scrollback so a runaway command cannot slow the launcher down.
- **Your shell is used** when it is zsh, bash, sh, dash or ksh; otherwise `/bin/sh`.

## Platform notes

| Platform | Shell                           |
| -------- | ------------------------------- |
| macOS    | Your login shell (zsh, bash, …) |
| Linux    | Your login shell (zsh, bash, …) |
| Windows  | `cmd.exe`                       |

Developed and tested on **macOS**; the Windows path is not yet tried on a real machine.

## Related

- [Web Search](./web-search.md) — Start with `$` to search the web.
