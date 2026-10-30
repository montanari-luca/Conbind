import { startFileDrag } from '../../lib/ipc/commands';
import { logService } from '../log/logService';
import { DRAG_THRESHOLD_PX } from './windowDragController';

const PREVIEW_SIZE = 64;

/** The path a result can be dragged out as: only apps and files stand for something on disk. */
export function draggablePath(item: { type?: string; path?: string }): string | undefined {
  return (item.type === 'application' || item.type === 'file') && item.path ? item.path : undefined;
}

/**
 * Renders the image shown under the cursor while a result is dragged: the
 * row's own icon when it is a loaded `<img>`, otherwise a neutral tile with the
 * title's first letter (emoji and built-in icons are not drawable from an
 * element). Returns bare base64 — the Rust side decodes it.
 */
export function renderDragPreview(node: HTMLElement): string {
  const canvas = document.createElement('canvas');
  canvas.width = PREVIEW_SIZE;
  canvas.height = PREVIEW_SIZE;
  const ctx = canvas.getContext('2d');
  if (!ctx) return '';

  const img = node.querySelector<HTMLImageElement>('img');
  try {
    if (img && img.complete && img.naturalWidth > 0) {
      ctx.drawImage(img, 0, 0, PREVIEW_SIZE, PREVIEW_SIZE);
      return canvas.toDataURL('image/png').split(',')[1] ?? '';
    }
  } catch {
    // A tainted canvas (cross-origin icon) cannot be exported; use the tile.
    ctx.clearRect(0, 0, PREVIEW_SIZE, PREVIEW_SIZE);
  }

  ctx.fillStyle = 'rgba(128, 128, 128, 0.85)';
  ctx.beginPath();
  ctx.roundRect(4, 4, PREVIEW_SIZE - 8, PREVIEW_SIZE - 8, 12);
  ctx.fill();
  ctx.fillStyle = '#fff';
  ctx.font = '600 28px sans-serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const label = node.querySelector('.result-title')?.textContent?.trim().charAt(0) ?? '';
  ctx.fillText(label.toUpperCase(), PREVIEW_SIZE / 2, PREVIEW_SIZE / 2 + 1);
  return canvas.toDataURL('image/png').split(',')[1] ?? '';
}

/**
 * Svelte action: pressing on the node and moving past the drag threshold hands
 * the file at `path` to the OS as a native drag. A falsy `path` disables it.
 *
 * Listens on `document` after pointerdown, not on the node: the cursor leaves
 * a row within a few pixels of travel. The click that would follow the drag is
 * swallowed so releasing over the row does not also run it.
 */
export function fileDrag(node: HTMLElement, path: string | undefined) {
  let current = path;
  let origin: { x: number; y: number } | null = null;
  let started = false;

  function stop() {
    origin = null;
    document.removeEventListener('pointermove', onMove);
    document.removeEventListener('pointerup', stop);
    document.removeEventListener('pointercancel', stop);
  }

  function onMove(e: PointerEvent) {
    if (!origin || started || !current) return;
    if (Math.hypot(e.clientX - origin.x, e.clientY - origin.y) < DRAG_THRESHOLD_PX) return;
    started = true;
    const target = current;
    stop();
    void startFileDrag(target, renderDragPreview(node)).catch((err) => {
      logService.warn(`Could not start file drag for ${target}: ${err}`);
    });
  }

  function onDown(e: PointerEvent) {
    if (e.button !== 0 || !current) return;
    started = false;
    origin = { x: e.clientX, y: e.clientY };
    document.addEventListener('pointermove', onMove);
    document.addEventListener('pointerup', stop);
    document.addEventListener('pointercancel', stop);
  }

  function onClickCapture(e: MouseEvent) {
    if (!started) return;
    started = false;
    e.stopPropagation();
    e.preventDefault();
  }

  node.addEventListener('pointerdown', onDown);
  node.addEventListener('click', onClickCapture, true);

  return {
    update(next: string | undefined) {
      current = next;
    },
    destroy() {
      stop();
      node.removeEventListener('pointerdown', onDown);
      node.removeEventListener('click', onClickCapture, true);
    },
  };
}
