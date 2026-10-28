import { describe, it, expect, vi } from 'vitest';
import { getAppDataDir, getDevExtensionsFile } from './platform';
import * as os from 'os';
import * as path from 'path';

describe('platform', () => {
  it('returns correctly formatted app data dir on darwin', () => {
    vi.stubGlobal('process', { ...process, platform: 'darwin' });
    const homedir = os.homedir();
    const expected = path.join(homedir, 'Library', 'Application Support', 'org.conbind');
    expect(getAppDataDir()).toBe(expected);
    vi.unstubAllGlobals();
  });

  it('returns dev flavor app data dir on darwin when isDevFlavor is true', () => {
    vi.stubGlobal('process', { ...process, platform: 'darwin' });
    const homedir = os.homedir();
    const expected = path.join(homedir, 'Library', 'Application Support', 'org.conbind.dev');
    expect(getAppDataDir(true)).toBe(expected);
    vi.unstubAllGlobals();
  });

  it('returns correctly formatted dev extensions file path', () => {
    vi.stubGlobal('process', { ...process, platform: 'darwin' });
    const homedir = os.homedir();
    const appDir = path.join(homedir, 'Library', 'Application Support', 'org.conbind');
    const expected = path.join(appDir, 'dev_extensions.json');
    expect(getDevExtensionsFile()).toBe(expected);
    vi.unstubAllGlobals();
  });

  it('returns dev extensions file path under dev flavor when isDevFlavor is true', () => {
    vi.stubGlobal('process', { ...process, platform: 'darwin' });
    const homedir = os.homedir();
    const appDir = path.join(homedir, 'Library', 'Application Support', 'org.conbind.dev');
    const expected = path.join(appDir, 'dev_extensions.json');
    expect(getDevExtensionsFile(true)).toBe(expected);
    vi.unstubAllGlobals();
  });
});
