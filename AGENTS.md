# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project

Conbind is a fast, local-first launcher for macOS, Windows and Linux, built with Tauri 2 (Rust) and SvelteKit 5. It has no accounts, no login, no cloud sync and no telemetry. Settings move between machines through **Settings → Backup** (export/import).

## Layout

| Path                   | What it is                                                              |
| ---------------------- | ----------------------------------------------------------------------- |
| `conbind-launcher/`    | The app: SvelteKit frontend (`src/`) and Tauri/Rust host (`src-tauri/`) |
| `conbind-sdk/`         | `conbind-sdk` npm package and the `conbind` extension-developer CLI     |
| `conbind-ext-builder/` | Sidecar for the AI extension builder (uses Bun)                         |
| `docs/`                | User guide, how-tos, reference and explanations                         |
| `scripts/`             | Repo-level dev, build, check and release scripts                        |

## Commands

Run from the repo root unless noted.

```bash
pnpm install            # install deps (builds conbind-sdk via its prepare script)
pnpm dev                # run the app in dev mode
pnpm check              # SDK doctor, design-system check, svelte-check
pnpm format:check       # Prettier (use `pnpm format` to fix)
pnpm gen:all            # regenerate generated files (diagnostic kinds, permission catalog)

cd conbind-launcher && pnpm exec vitest run          # frontend tests
cd conbind-launcher/src-tauri && cargo test --workspace   # Rust tests
cd conbind-launcher/src-tauri && cargo fmt           # Rust formatting
cd conbind-sdk && pnpm exec vitest run               # SDK tests
```

## Conventions

- Match the surrounding code's style, naming and comment density.
- Format with Prettier (TS/Svelte/JSON/Markdown) and `cargo fmt` (Rust) before finishing.
- Do not hand-edit generated files (`src/services/diagnostics/kinds.ts`, both `gatedPermissions.ts`, `bindings.ts`); change the source and run `pnpm gen:all`.
- Tauri commands are registered in `conbind-launcher/src-tauri/src/commands/handlers.rs`; add the Rust command, register it, then add a typed wrapper under `conbind-launcher/src/lib/ipc/`.
- User-visible strings go through `t()` and must be added to all four files in `conbind-launcher/src/locales/`.
- Do not add network calls that send user data off-device, and do not reintroduce accounts, sync or telemetry.
- The extension store catalog still lives at `asyar.org`; that host is intentionally left as-is.
- Suggest a commit message when done

## Documentation

- Make sure to update the `README.md` file accordingly after any change that modifies something mentioned in it.
