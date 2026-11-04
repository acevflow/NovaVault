/**
 * Errors returned by Tauri commands.
 *
 * The shell rejects a failed command with `{ code, message }`, where the
 * message is a fixed English string that never contains secrets. The UI
 * branches on `code` and shows its own wording from {@link errorMessage}.
 */

/** Every error code the shell can return; mirrors `CommandError::code()`. */
export const ERROR_CODES = [
  'locked',
  'vault_not_found',
  'vault_exists',
  'invalid_credentials',
  'invalid_recovery_code',
  'invalid_input',
  'item_not_found',
  'invalid_item',
  'unsupported_vault',
  'corrupted',
  'storage',
  'internal',
] as const;

/** A machine-readable error code from the shell. */
export type ErrorCode = (typeof ERROR_CODES)[number];

/** A failed Tauri command. */
export class CommandError extends Error {
  /** Stable code to branch on. */
  readonly code: ErrorCode;

  /**
   * @param code - The error code reported by the shell.
   * @param message - The shell's fixed message, for diagnostics.
   */
  constructor(code: ErrorCode, message: string) {
    super(message);
    this.name = 'CommandError';
    this.code = code;
  }
}

/** Whether `value` is a {@link CommandError}, optionally with a given code. */
export function isCommandError(value: unknown, code?: ErrorCode): value is CommandError {
  return value instanceof CommandError && (code === undefined || value.code === code);
}

function isErrorCode(value: unknown): value is ErrorCode {
  return typeof value === 'string' && (ERROR_CODES as readonly string[]).includes(value);
}

/**
 * Converts whatever `invoke` rejected with into a {@link CommandError}.
 *
 * Anything that is not a well-formed shell error (an IPC failure, a code
 * added by a newer shell) becomes `internal`, so callers handle one type.
 */
export function toCommandError(reason: unknown): CommandError {
  if (reason instanceof CommandError) {
    return reason;
  }
  if (typeof reason === 'object' && reason !== null && 'code' in reason) {
    const { code } = reason;
    const message = 'message' in reason && typeof reason.message === 'string' ? reason.message : '';
    if (isErrorCode(code)) {
      return new CommandError(code, message);
    }
  }
  return new CommandError('internal', 'unexpected command failure');
}

const MESSAGES: Record<ErrorCode, string> = {
  locked: 'The vault is locked. Unlock it to continue.',
  vault_not_found: 'No vault was found. Create a new vault to get started.',
  vault_exists: 'A vault already exists on this device.',
  invalid_credentials: 'That password or recovery code is not correct.',
  invalid_recovery_code:
    'That recovery code is not valid. Check it for typos: it has 8 groups of 4 characters.',
  invalid_input: 'Some of the information entered is not valid.',
  item_not_found: 'That item no longer exists.',
  invalid_item: 'The item could not be saved. It may be too large.',
  unsupported_vault:
    'The vault file is not supported by this version of NovaVault. You may need a newer version.',
  corrupted: 'The vault file is damaged and cannot be opened.',
  storage: 'The vault file could not be read or written. Check the disk and try again.',
  internal: 'Something went wrong. Please try again.',
};

/** User-facing text for an error, suitable for an alert. */
export function errorMessage(error: unknown): string {
  return MESSAGES[toCommandError(error).code];
}
