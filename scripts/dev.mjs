#!/usr/bin/env node
/**
 * Build the SDK then start the Conbind app in development mode.
 * Cross-platform (Node.js, no bash dependencies).
 */
import { execSync, spawn } from 'child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

console.log('Building conbind-sdk once to initialize dist/...');
try {
  execSync('pnpm run build:all', {
    cwd: resolve(root, 'conbind-sdk'),
    stdio: 'inherit',
  });
} catch {
  process.exit(1);
}

console.log('Starting development mode (SDK Watch + Launcher)...');

const sdkWatch = spawn('pnpm', ['run', 'watch'], {
  cwd: resolve(root, 'conbind-sdk'),
  stdio: 'inherit',
  shell: true,
});

// --config merges tauri.dev.conf.json (JSON Merge Patch) over tauri.conf.json,
// giving the dev build its own identifier (org.conbind.dev) so it gets a fully
// separate app data dir from the installed production app — extensions,
// snippets, search index, and caches are all isolated.
const launcherDev = spawn('pnpm', ['tauri', 'dev', '--config', 'src-tauri/tauri.dev.conf.json'], {
  cwd: resolve(root, 'conbind-launcher'),
  stdio: 'inherit',
  shell: true,
});

// Ensure both exit when one stops
const cleanup = (code) => {
  sdkWatch.kill();
  launcherDev.kill();
  process.exit(code ?? 0);
};

sdkWatch.on('exit', cleanup);
launcherDev.on('exit', cleanup);
process.on('SIGINT', cleanup);
process.on('SIGTERM', cleanup);
