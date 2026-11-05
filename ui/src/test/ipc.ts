/**
 * Test helpers that fake the Tauri shell with `mockIPC`.
 */
import { mockIPC } from '@tauri-apps/api/mocks';

import type { CommandName } from '../api/commands';
import type { ErrorCode } from '../api/errors';

/** Arguments a command was called with. */
export type CommandArgs = Record<string, unknown>;

/** Fake command implementations. A handler may return a value or a promise, or throw. */
export type CommandHandlers = Partial<Record<CommandName, (args: CommandArgs) => unknown>>;

/** One recorded command call. */
export interface RecordedCall {
  readonly command: string;
  readonly args: CommandArgs;
}

/**
 * Installs fake commands and returns the list of calls they receive.
 * Commands without a handler reject with `internal`, so tests notice
 * unexpected calls.
 */
export function mockCommands(handlers: CommandHandlers): RecordedCall[] {
  const calls: RecordedCall[] = [];
  mockIPC((command, payload) => {
    const args = (payload ?? {}) as CommandArgs;
    calls.push({ command, args });
    const handler = handlers[command as CommandName];
    if (handler === undefined) {
      throw shellError('internal');
    }
    return handler(args);
  });
  return calls;
}

/**
 * A rejection shaped like the shell's `{ code, message }`. It extends
 * `Error` only so tests can `throw` it; the UI reads just the two fields.
 */
export class ShellError extends Error {
  /** The error code. */
  readonly code: ErrorCode;

  /** @param code - The error code to reject with. */
  constructor(code: ErrorCode) {
    super(`shell error: ${code}`);
    this.code = code;
  }
}

/** The rejection the shell sends for `code`. */
export function shellError(code: ErrorCode): ShellError {
  return new ShellError(code);
}

/** A recovery code in display form, for tests. */
export const TEST_RECOVERY_CODE = 'ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6789';
