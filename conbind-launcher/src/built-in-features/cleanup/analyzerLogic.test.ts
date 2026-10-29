import { describe, expect, it } from 'vitest';
import type { AnalyzedDir } from '../../lib/ipc/commands';
import { entryFraction, removeFromListings } from './analyzerLogic';

const home: AnalyzedDir = {
  path: '/home/u',
  parent: null,
  totalBytes: 1000,
  entries: [
    { name: 'proj', path: '/home/u/proj', isDir: true, bytes: 800 },
    { name: 'notes.txt', path: '/home/u/notes.txt', isDir: false, bytes: 200 },
  ],
};
const proj: AnalyzedDir = {
  path: '/home/u/proj',
  parent: '/home/u',
  totalBytes: 800,
  entries: [
    { name: 'big.bin', path: '/home/u/proj/big.bin', isDir: false, bytes: 500 },
    { name: 'src', path: '/home/u/proj/src', isDir: true, bytes: 300 },
  ],
};

describe('analyzerLogic', () => {
  it('drops the trashed entry and shrinks every ancestor', () => {
    const [h, p] = removeFromListings([home, proj], '/home/u/proj/big.bin', 500);
    expect(p.entries.map((e) => e.name)).toEqual(['src']);
    expect(p.totalBytes).toBe(300);
    expect(h.totalBytes).toBe(500);
    expect(h.entries.find((e) => e.name === 'proj')?.bytes).toBe(300);
    expect(h.entries.find((e) => e.name === 'notes.txt')?.bytes).toBe(200);
  });

  it('removes a directory entry from its parent listing', () => {
    const [h] = removeFromListings([home], '/home/u/proj', 800);
    expect(h.entries.map((e) => e.name)).toEqual(['notes.txt']);
    expect(h.totalBytes).toBe(200);
  });

  it('leaves unrelated listings and sibling prefixes alone', () => {
    const sibling: AnalyzedDir = { ...proj, path: '/home/u/proj2', totalBytes: 50, entries: [] };
    const [out] = removeFromListings([sibling], '/home/u/proj/big.bin', 500);
    expect(out).toBe(sibling);
  });

  it('never goes negative', () => {
    const [h] = removeFromListings([home], '/home/u/notes.txt', 99999);
    expect(h.totalBytes).toBe(0);
  });

  it('measures rows against the largest entry', () => {
    expect(entryFraction(home, 400)).toBeCloseTo(0.5);
    expect(entryFraction({ ...home, entries: [] }, 10)).toBe(0);
  });
});
