import { startFileDrag } from '../../lib/ipc/commands';
import { logService } from '../log/logService';
import { DRAG_THRESHOLD_PX } from './windowDragController';

const PREVIEW_SIZE = 64;

/** The path a result can be dragged out as: only apps and files stand for something on disk. */
export function draggablePath(item: { type?: string; path?: string }): string | undefined {
  return (item.type === 'application' || item.type === 'file') && item.path ? item.path : undefined;
}

function newCanvas(): { canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D } | null {
  const canvas = document.createElement('canvas');
  canvas.width = PREVIEW_SIZE;
  canvas.height = PREVIEW_SIZE;
  const ctx = canvas.getContext('2d');
  return ctx ? { canvas, ctx } : null;
}

function pngBase64(canvas: HTMLCanvasElement): string {
  return canvas.toDataURL('image/png').split(',')[1] ?? '';
}

/**
 * A neutral tile with the title's first letter. The fallback for icons that
 * cannot be drawn (emoji, built-in icons, an image that has not loaded).
 * Returns bare base64 — the Rust side decodes it.
 */
export function renderTilePreview(node: HTMLElement): string {
  const target = newCanvas();
  if (!target) return '';
  const { canvas, ctx } = target;
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
  return pngBase64(canvas);
}

/**
 * Renders the row's own icon as the drag image, or resolves to `null` when it
 * cannot. The icon is re-loaded with `crossOrigin` rather than drawn from the
 * row's `<img>`: that element came from a custom scheme without CORS mode, so
 * drawing it taints the canvas and `toDataURL` throws a SecurityError. The
 * icon schemes send `Access-Control-Allow-Origin: *`, so the anonymous load is
 * clean.
 */
export function loadIconPreview(node: HTMLElement): Promise<string | null> {
  const src = node.querySelector<HTMLImageElement>('img')?.src;
  if (!src) return Promise.resolve(null);
  return new Promise((resolve) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      try {
        const target = newCanvas();
        if (!target) return resolve(null);
        target.ctx.drawImage(img, 0, 0, PREVIEW_SIZE, PREVIEW_SIZE);
        resolve(pngBase64(target.canvas));
      } catch {
        resolve(null);
      }
    };
    img.onerror = () => resolve(null);
    img.src = src;
  });
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
  let iconPreview: string | null = null;

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
    void startFileDrag(target, iconPreview ?? renderTilePreview(node)).catch((err) => {
      logService.warn(`Could not start file drag for ${target}: ${err}`);
    });
  }

  function onDown(e: PointerEvent) {
    if (e.button !== 0 || !current) return;
    started = false;
    iconPreview = null;
    // Loaded now so it is ready by the time the pointer crosses the threshold:
    // the native drag has to start synchronously from the move event.
    void loadIconPreview(node).then((png) => {
      iconPreview = png;
    });
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
