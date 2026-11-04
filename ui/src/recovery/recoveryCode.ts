/**
 * Helpers for showing and re-typing recovery codes in the UI.
 *
 * The shell parses and validates codes; these only compare what the user
 * retyped with the code just shown, before the flow can finish.
 */

/** Uppercases and keeps only letters and digits, like the shell's parser. */
export function normalizeRecoveryCode(text: string): string {
  return text.toUpperCase().replace(/[^0-9A-Z]/g, '');
}

/** Whether `typed` is the same code as `shown`, ignoring case, spaces and dashes. */
export function recoveryCodesMatch(shown: string, typed: string): boolean {
  const expected = normalizeRecoveryCode(shown);
  return expected !== '' && expected === normalizeRecoveryCode(typed);
}

/** Splits a display-form code (`XXXX-XXXX-…`) into its groups. */
export function recoveryCodeGroups(code: string): string[] {
  return code.split('-').filter((group) => group !== '');
}
