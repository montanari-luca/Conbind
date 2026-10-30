import { describe, it, expect } from 'vitest';
import { displayCwd, parseTerminalQuery } from './terminalLogic';

describe('parseTerminalQuery', () => {
  it('reads the command after > with or without a space', () => {
    expect(parseTerminalQuery('>ls -la')).toBe('ls -la');
    expect(parseTerminalQuery('> cd /tmp')).toBe('cd /tmp');
    expect(parseTerminalQuery('  >  pwd  ')).toBe('pwd');
  });

  it('ignores queries without the prefix or without a command', () => {
    expect(parseTerminalQuery('ls')).toBeNull();
    expect(parseTerminalQuery('>')).toBeNull();
    expect(parseTerminalQuery('>   ')).toBeNull();
  });
});

describe('displayCwd', () => {
  it('shortens the home directory', () => {
    expect(displayCwd('/Users/a', '/Users/a')).toBe('~');
    expect(displayCwd('/Users/a/dev', '/Users/a/')).toBe('~/dev');
    expect(displayCwd('C:\\Users\\a\\dev', 'C:\\Users\\a')).toBe('~\\dev');
  });

  it('leaves other paths alone', () => {
    expect(displayCwd('/tmp', '/Users/a')).toBe('/tmp');
    expect(displayCwd('/Users/ab', '/Users/a')).toBe('/Users/ab');
    expect(displayCwd('/tmp', null)).toBe('/tmp');
  });
});
