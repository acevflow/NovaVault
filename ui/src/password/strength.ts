/**
 * Master password strength estimation with zxcvbn-ts.
 *
 * The estimator and its dictionaries (about 1.5 MB) are loaded on first
 * use with dynamic imports, so they never slow down startup or the unlock
 * screen. Only the score and feedback leave this module; the result's echo
 * of the password is dropped.
 */
import type { Score, ZxcvbnFactory } from '@zxcvbn-ts/core';

/** zxcvbn score: 0 (too guessable) to 4 (very unguessable). */
export type StrengthScore = Score;

/**
 * Lowest score accepted for a master password: "safely unguessable",
 * about 10^10 guesses, which resists offline attacks together with the
 * Argon2id cost of every guess.
 */
export const MIN_ACCEPTED_SCORE: StrengthScore = 3;

/** Result of {@link estimateStrength}. */
export interface StrengthEstimate {
  /** The score. */
  readonly score: StrengthScore;
  /** Main problem with the password, if any. */
  readonly warning: string | null;
  /** How to make it stronger. */
  readonly suggestions: readonly string[];
}

/** Display label for each score. */
export const STRENGTH_LABELS: Readonly<Record<StrengthScore, string>> = {
  0: 'Very weak',
  1: 'Weak',
  2: 'Fair',
  3: 'Strong',
  4: 'Very strong',
};

let estimator: Promise<ZxcvbnFactory> | null = null;

async function loadEstimator(): Promise<ZxcvbnFactory> {
  const [{ ZxcvbnFactory }, common, english] = await Promise.all([
    import('@zxcvbn-ts/core'),
    import('@zxcvbn-ts/language-common'),
    import('@zxcvbn-ts/language-en'),
  ]);
  return new ZxcvbnFactory({
    dictionary: { ...common.dictionary, ...english.dictionary },
    graphs: common.adjacencyGraphs,
    translations: english.translations,
  });
}

/**
 * Estimates how hard `password` is to guess.
 *
 * @param userInputs - Words the password should not be based on (such as the app name).
 */
export async function estimateStrength(
  password: string,
  userInputs: readonly string[] = ['novavault', 'nova', 'vault'],
): Promise<StrengthEstimate> {
  estimator ??= loadEstimator().catch((error: unknown) => {
    // Allow a retry on the next call instead of caching the failure.
    estimator = null;
    throw error;
  });
  const result = (await estimator).check(password, [...userInputs]);
  return {
    score: result.score,
    warning: result.feedback.warning,
    suggestions: result.feedback.suggestions,
  };
}
