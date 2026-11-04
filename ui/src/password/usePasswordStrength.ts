import { useEffect, useState } from 'react';

import { estimateStrength, type StrengthEstimate } from './strength';

/**
 * Strength of `password`, re-estimated as it changes.
 *
 * @returns The estimate, or `null` while the password is empty or the
 *   estimate is not ready yet.
 */
export function usePasswordStrength(password: string): StrengthEstimate | null {
  const [result, setResult] = useState<{
    readonly password: string;
    readonly estimate: StrengthEstimate;
  } | null>(null);

  useEffect(() => {
    if (password === '') {
      return;
    }
    let current = true;
    estimateStrength(password)
      .then((estimate) => {
        // Ignore answers for a value the user has already changed.
        if (current) setResult({ password, estimate });
      })
      .catch(() => {
        // Estimation failed (the dictionaries could not load): no feedback,
        // and callers keep the submit button disabled.
      });
    return () => {
      current = false;
    };
  }, [password]);

  // Only return an estimate that matches the current value, so a stale
  // "strong" result never enables submitting a weak password.
  return result !== null && result.password === password ? result.estimate : null;
}
