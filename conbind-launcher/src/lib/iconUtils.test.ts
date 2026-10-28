import { describe, expect, it } from 'vitest';
import { isIconImage } from './iconUtils';

describe('isIconImage', () => {
  it('treats an conbind-thumb URL as an image icon', () => {
    expect(isIconImage('conbind-thumb://localhost/native.png')).toBe(true);
  });
});
