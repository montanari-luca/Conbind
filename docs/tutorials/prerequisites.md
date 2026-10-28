---
order: 1
---

## 2. Prerequisites & Environment Setup

### Runtime requirements

| Requirement | Minimum Version          |
| ----------- | ------------------------ |
| Node.js     | 18 or later              |
| pnpm        | 8 or later (recommended) |
| Conbind app | Installed and running    |

### Install the Conbind CLI

The `conbind-sdk` npm package provides both the runtime SDK and the `conbind` CLI used throughout your entire development workflow.

```bash
npm install -g conbind-sdk
```

Verify installation:

```bash
conbind --version
```

### Per-project dependencies

Every extension project needs at minimum:

```bash
pnpm add conbind-sdk svelte
pnpm add -D vite @sveltejs/vite-plugin-svelte typescript
```

The CLI scaffolder installs these automatically. You only need to run this manually if you are setting up a project by hand.
