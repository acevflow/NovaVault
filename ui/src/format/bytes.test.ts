import { describe, expect, it } from 'vitest';

import { formatBytes, percentDone } from './bytes';

describe('formatBytes', () => {
  it('uses bytes below one kilobyte', () => {
    expect(formatBytes(0)).toBe('0 bytes');
    expect(formatBytes(1)).toBe('1 byte');
    expect(formatBytes(1023)).toBe('1023 bytes');
  });

  it('uses one decimal below 10 and none above', () => {
    expect(formatBytes(1024)).toBe('1.0 KB');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(12 * 1024 * 1024)).toBe('12 MB');
    expect(formatBytes(3.25 * 1024 ** 3)).toBe('3.3 GB');
  });

  it('stops at terabytes', () => {
    expect(formatBytes(2048 * 1024 ** 4)).toBe('2048 TB');
  });
});

describe('percentDone', () => {
  it('rounds down and caps at 100', () => {
    expect(percentDone(0, 100)).toBe(0);
    expect(percentDone(999, 1000)).toBe(99);
    expect(percentDone(1000, 1000)).toBe(100);
    expect(percentDone(1200, 1000)).toBe(100);
  });

  it('treats an empty transfer as done', () => {
    expect(percentDone(0, 0)).toBe(100);
  });
});
