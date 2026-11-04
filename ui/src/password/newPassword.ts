/**
 * Validation of a new master password (creation and password change).
 */
import { MIN_ACCEPTED_SCORE, STRENGTH_LABELS, type StrengthEstimate } from './strength';

/** Why a new password cannot be used yet. */
export type NewPasswordProblem = 'empty' | 'checking' | 'weak' | 'mismatch';

/** Why a new password cannot be used yet, or `null` if it can. */
export function newPasswordProblem(
  password: string,
  confirmation: string,
  estimate: StrengthEstimate | null,
): NewPasswordProblem | null {
  if (password === '') {
    return 'empty';
  }
  if (estimate === null) {
    return 'checking';
  }
  if (estimate.score < MIN_ACCEPTED_SCORE) {
    return 'weak';
  }
  if (password !== confirmation) {
    return 'mismatch';
  }
  return null;
}

/**
 * Message for a problem that is not shown next to a field. A mismatch is
 * shown on the confirmation field instead, so it has no message here.
 */
export function newPasswordProblemMessage(problem: NewPasswordProblem | null): string | null {
  switch (problem) {
    case 'empty':
      return 'Enter a master password.';
    case 'checking':
      return 'Password strength is still being checked. Try again in a moment.';
    case 'weak':
      return `The password must rate at least “${STRENGTH_LABELS[MIN_ACCEPTED_SCORE]}”. Make it longer or add unrelated words.`;
    case 'mismatch':
    case null:
      return null;
  }
}
