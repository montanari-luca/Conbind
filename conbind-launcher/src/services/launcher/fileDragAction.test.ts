// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../../lib/ipc/commands', () => ({
  startFileDrag: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('../log/logService', () => ({ logService: { warn: vi.fn() } }));

import { draggablePath, fileDrag } from './fileDragAction';
import { DRAG_THRESHOLD_PX } from './windowDragController';
import { startFileDrag } from '../../lib/ipc/commands';

function pointer(type: string, x: number, y: number, button = 0) {
  const e = new Event(type, { bubbles: true, cancelable: true }) as any;
  e.clientX = x;
  e.clientY = y;
  e.button = button;
  return e as PointerEvent;
}

describe('draggablePath', () => {
  it('returns the path for apps and files only', () => {
    expect(draggablePath({ type: 'application', path: '/Applications/Foo.app' })).toBe(
      '/Applications/Foo.app',
    );
    expect(draggablePath({ type: 'file', path: '/tmp/a.txt' })).toBe('/tmp/a.txt');
    expect(draggablePath({ type: 'command', path: '/tmp/a.txt' })).toBeUndefined();
    expect(draggablePath({ type: 'file' })).toBeUndefined();
  });
});

describe('fileDrag action', () => {
  let node: HTMLButtonElement;

  beforeEach(() => {
    vi.mocked(startFileDrag).mockClear();
    node = document.createElement('button');
    document.body.appendChild(node);
  });

  it('starts a drag once the pointer passes the threshold', () => {
    fileDrag(node, '/tmp/a.txt');
    node.dispatchEvent(pointer('pointerdown', 10, 10));
    document.dispatchEvent(pointer('pointermove', 10, 10 + DRAG_THRESHOLD_PX - 1));
    expect(startFileDrag).not.toHaveBeenCalled();
    document.dispatchEvent(pointer('pointermove', 10, 10 + DRAG_THRESHOLD_PX + 1));
    document.dispatchEvent(pointer('pointermove', 10, 50));
    expect(startFileDrag).toHaveBeenCalledTimes(1);
    expect(vi.mocked(startFileDrag).mock.calls[0][0]).toBe('/tmp/a.txt');
  });

  it('does nothing without a path or for a non-primary button', () => {
    const action = fileDrag(node, undefined);
    node.dispatchEvent(pointer('pointerdown', 0, 0));
    document.dispatchEvent(pointer('pointermove', 50, 50));
    expect(startFileDrag).not.toHaveBeenCalled();

    action.update('/tmp/a.txt');
    node.dispatchEvent(pointer('pointerdown', 0, 0, 2));
    document.dispatchEvent(pointer('pointermove', 50, 50));
    expect(startFileDrag).not.toHaveBeenCalled();
  });

  it('does not drag after the pointer is released', () => {
    fileDrag(node, '/tmp/a.txt');
    node.dispatchEvent(pointer('pointerdown', 0, 0));
    document.dispatchEvent(pointer('pointerup', 0, 0));
    document.dispatchEvent(pointer('pointermove', 50, 50));
    expect(startFileDrag).not.toHaveBeenCalled();
  });

  it('swallows the click that follows a drag', () => {
    fileDrag(node, '/tmp/a.txt');
    const onclick = vi.fn();
    node.addEventListener('click', onclick);
    node.dispatchEvent(pointer('pointerdown', 0, 0));
    document.dispatchEvent(pointer('pointermove', 50, 50));
    node.click();
    expect(onclick).not.toHaveBeenCalled();
    node.click();
    expect(onclick).toHaveBeenCalledTimes(1);
  });
});
