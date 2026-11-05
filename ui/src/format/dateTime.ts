/**
 * Human-readable dates and times for UI text.
 */

/** A timestamp (milliseconds since the Unix epoch) as a medium date and short time. */
export function formatDateTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}
