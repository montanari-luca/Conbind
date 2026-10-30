/** The text after the `>` prefix, or `null` when `query` is not a terminal command. */
export function parseTerminalQuery(query: string): string | null {
  const trimmed = query.trim();
  if (!trimmed.startsWith('>')) return null;
  const command = trimmed.slice(1).trim();
  return command || null;
}

/** Shortens the home directory to `~` for display. */
export function displayCwd(cwd: string, home: string | null): string {
  if (!home) return cwd;
  const trimmedHome = home.replace(/[\\/]+$/, '');
  if (cwd === trimmedHome) return '~';
  if (cwd.startsWith(trimmedHome + '/') || cwd.startsWith(trimmedHome + '\\')) {
    return '~' + cwd.slice(trimmedHome.length);
  }
  return cwd;
}
