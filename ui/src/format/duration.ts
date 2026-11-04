/**
 * Human-readable durations for UI text.
 */

/** "1 second", "30 seconds", "1 minute", "2 minutes" (whole minutes only). */
export function formatSeconds(seconds: number): string {
  if (seconds >= 60 && seconds % 60 === 0) {
    const minutes = seconds / 60;
    return minutes === 1 ? '1 minute' : `${String(minutes)} minutes`;
  }
  return seconds === 1 ? '1 second' : `${String(seconds)} seconds`;
}
