// conbind-launcher/src/lib/ipc/fileCommands.ts
// Tauri command wrappers, re-exported through ./commands (the barrel).
import { invokeSafe } from './invokeSafe';

// ── File I/O ──────────────────────────────────────────────────────────────────

export async function checkPathExists(path: string): Promise<boolean> {
  return invokeSafe<boolean>('check_path_exists', { path });
}

export async function readTextFileAbsolute(pathStr: string): Promise<string> {
  return invokeSafe<string>('read_text_file_absolute', { pathStr });
}

export async function writeTextFileAbsolute(pathStr: string, content: string): Promise<void> {
  await invokeSafe('write_text_file_absolute', { pathStr, content });
}

export async function writeBinaryFileRecursive(pathStr: string, content: number[]): Promise<void> {
  await invokeSafe('write_binary_file_recursive', { pathStr, content });
}

export async function mkdirAbsolute(pathStr: string): Promise<void> {
  await invokeSafe('mkdir_absolute', { pathStr });
}

export async function showInFileManager(pathStr: string): Promise<void> {
  await invokeSafe('show_in_file_manager', { pathStr });
}

/** Starts a native drag of `pathStr` out of the window; `previewPng` is the base64 PNG under the cursor. */
export async function startFileDrag(pathStr: string, previewPng: string): Promise<void> {
  await invokeSafe('start_file_drag', { pathStr, previewPng });
}

export async function trashPath(pathStr: string): Promise<void> {
  await invokeSafe('trash_path', { pathStr });
}
