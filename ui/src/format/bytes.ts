/**
 * Human-readable file sizes for UI text.
 */

const UNITS = ['KB', 'MB', 'GB', 'TB'] as const;

/**
 * A size in bytes as text, in 1024-based units: "0 bytes", "1 byte",
 * "512 bytes", "1.5 KB", "12 MB". One decimal below 10, none above.
 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return bytes === 1 ? '1 byte' : `${String(bytes)} bytes`;
  }
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const rounded = value < 10 ? value.toFixed(1) : String(Math.round(value));
  return `${rounded} ${UNITS[unit] ?? 'TB'}`;
}

/** How far a transfer is, as a whole percentage from 0 to 100. */
export function percentDone(done: number, total: number): number {
  if (total <= 0) {
    return 100;
  }
  return Math.min(100, Math.floor((done / total) * 100));
}
