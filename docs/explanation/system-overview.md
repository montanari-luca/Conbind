---
order: 7
---

## 1. System Overview

Conbind is an extensible desktop launcher and productivity tool built on the Rust-based Tauri framework and web technologies. The core architectural challenge Conbind solves is providing a unified search and command execution interface capable of running both deeply integrated native features (Tier 1) and isolated third-party extensions (Tier 2) without compromising host security or user experience.

```ascii
+-----------------------------------------------------------------------------------+
|                                 Operating System (macOS / Windows / Linux)        |
+-----------------------------------------------------------------------------------+
      | Global Hotkeys        | Tray Menu       | File System       | Clipboard
+-----------------------------------------------------------------------------------+
|                                   Rust Host (Tauri 2)                             |
|  - conbind_lib::run()                                                               |
|  - Custom Protocol: conbind-extension://                                            |
|  - Global Shortcuts, Window Management, Notifications                             |
+-----------------------------------------------------------------------------------+
      | Tauri IPC (`@tauri-apps/api/core`)
+-----------------------------------------------------------------------------------+
|                            Privileged Host Context (WebView)                      |
|                                                                                   |
|  SvelteKit Frontend (`src/routes/+page.svelte`)                                   |
|  - ExtensionManager (`extensionManager.ts`)                                       |
|  - ExtensionLoaderService (`extensionLoaderService.ts`)                           |
|  - SearchService & Command Index                                                  |
|                                                                                   |
|  +---------------------------------------+                                        |
|  |     Tier 1: Built-in Features       |                                        |
|  |     (Shared JS Context)               |                                        |
|  |     e.g., Clipboard History, Store    |                                        |
|  +---------------------------------------+                                        |
|       | conbind-sdk proxy        ^                                                  |
|       v                        |                                                  |
|  +---------------------------------------+    +--------------------------------+  |
|  |        ExtensionIframe.svelte         |    |   Tier 2: Installed Extensions |  |
|  |        (IPC Bridge / postMessage)     |<---|   (Isolated Iframe Sandbox)    |  |
|  +---------------------------------------+    +--------------------------------+  |
+-----------------------------------------------------------------------------------+
```

---
