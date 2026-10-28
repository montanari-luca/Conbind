import { describe, it, expect, beforeEach } from 'vitest';
import { registerIconElement, ConbindIconElement } from './ConbindIconElement';

describe('ConbindIconElement', () => {
  beforeEach(() => {
    // Usually registerIconElement() is safe to call multiple times as it checks if it's already defined
    registerIconElement('conbind-icon-test');
  });

  it('renders SVG internally when name is set', () => {
    const el = document.createElement('conbind-icon-test') as ConbindIconElement;
    el.setAttribute('name', 'calculator');
    document.body.appendChild(el);

    expect(el.innerHTML).toContain('<svg');
    expect(el.innerHTML).toContain('width="20"');
    expect(el.style.display).toBe('inline-flex');
  });

  it('updates when attributes change', () => {
    const el = document.createElement('conbind-icon-test') as ConbindIconElement;
    el.setAttribute('name', 'calculator');
    el.setAttribute('size', '16');
    document.body.appendChild(el);

    expect(el.innerHTML).toContain('width="16"');

    el.setAttribute('size', '32');
    expect(el.innerHTML).toContain('width="32"');
  });

  it('applies default size if missing', () => {
    const el = document.createElement('conbind-icon-test') as ConbindIconElement;
    el.setAttribute('name', 'calculator');
    document.body.appendChild(el);

    expect(el.innerHTML).toContain('width="20"');
  });
});
