# Explanation

Understanding-oriented background. These pages describe how Conbind works and why — useful when you need to reason about behaviour, not just invoke an API.

## Pages in this section

### Conceptual overview

- **[Introduction](./introduction.md)** — What Conbind is, what an extension can do, the sandbox model, why Svelte 5 + Vite.

### Extension architecture (for extension authors)

- **[Two-tier model](./two-tier-model.md)** — Built-in features vs installed extensions. Why the two-tier split exists.
- **[IPC bridge](./ipc-bridge.md)** — How service calls travel from an iframe to the host and back.
- **[conbind-extension:// protocol](./conbind-extension-protocol.md)** — How iframe URLs resolve to files on disk.
- **[Permission system](./permission-system.md)** — Iframe sandbox, CSP, and the two-layer permission check.
- **[Lifecycle](./lifecycle.md)** — Discovery → initialize → activate → active → deactivate.

### Launcher internals (for contributors)

- **[System overview](./system-overview.md)** — The Rust host, SvelteKit WebView, and the custom protocol.
- **[Technology stack](./technology-stack.md)** — Tauri 2, SvelteKit, conbind-sdk, the custom protocol.
- **[Host startup, installation & view rendering](./host-startup-and-installation.md)** — What happens from app launch to a rendered view.
- **[Host OS integration & data flow](./host-os-integration.md)** — OS-level hooks and cross-layer data flow diagrams.
- **[Launcher rendering lifecycle](./launcher-rendering-lifecycle.md)** — How the launcher stays flicker-free on macOS: the pinned webview, the parked panel, and paint-synchronized resizes.
- **[Data persistence](./data-persistence.md)** — Where things are stored and known limitations.
- **[AI Extension Builder](./ai-extension-builder.md)** — How "Build Extension with AI" works: the Tier-1 orchestrator ⇄ coding sidecar, the stdio protocol, feasibility gate, security model, and the claude-runtime packaging constraint.
- **[File Search](./file-search.md)** — The Rust-owned filename index and thumbnail cache: the arena, the bounded query engine, the parallel walker, and the coalesced filesystem watcher.
- **[Locale Subsystem & Internationalization](./locale-and-internationalization.md)** — The centralized Rust-first locale architecture: BCP-47 parsing, candidate fallback chains, Language-First text resolution vs. Region-First CLDR number notation, and LocaleService state management.
- **[Browser Bridge Protocol](./browser-bridge-protocol.md)** — The wire contract between the launcher's local WebSocket server and companion browser extensions: pairing, auth, and the RPC surface.
