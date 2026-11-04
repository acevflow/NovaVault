/**
 * Typed wrappers over the Tauri commands: the UI's whole IPC surface.
 *
 * Each function calls one command declared in `src-tauri/build.rs` and
 * rejects with a {@link CommandError} on failure. Secrets passed here go
 * straight to Rust; the UI never receives key material, only the recovery
 * code at the two moments it must be shown.
 */
import { invoke, type InvokeArgs } from '@tauri-apps/api/core';

import { toCommandError, type CommandError } from './errors';
import type { Credential, RecoveryCodeResponse, Settings, VaultStatus } from './types';

/** Names of the commands this UI calls. */
export type CommandName =
  | 'vault_status'
  | 'create_vault'
  | 'unlock'
  | 'lock'
  | 'change_master_password'
  | 'regenerate_recovery_code'
  | 'get_settings'
  | 'set_auto_lock_timeout';

async function call<T>(command: CommandName, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (reason) {
    throw toCommandError(reason) satisfies CommandError;
  }
}

/** Reports whether a vault exists and whether it is unlocked. */
export function vaultStatus(): Promise<VaultStatus> {
  return call('vault_status');
}

/**
 * Creates the vault protected by `password` and leaves it unlocked.
 *
 * @returns The recovery code, to show to the user exactly once.
 */
export async function createVault(password: string): Promise<string> {
  const response = await call<RecoveryCodeResponse>('create_vault', { password });
  return response.recoveryCode;
}

/** Unlocks the vault with the master password or the recovery code. */
export function unlock(credential: Credential): Promise<void> {
  return call('unlock', { credential });
}

/** Locks the vault; the shell wipes the vault key. */
export function lock(): Promise<void> {
  return call('lock');
}

/**
 * Changes the master password.
 *
 * @param currentCredential - The current password, or the recovery code if it is forgotten.
 * @param newPassword - The new master password.
 */
export function changeMasterPassword(
  currentCredential: Credential,
  newPassword: string,
): Promise<void> {
  return call('change_master_password', { currentCredential, newPassword });
}

/**
 * Replaces the recovery code; the previous one stops working.
 *
 * @param currentCredential - The current password or recovery code.
 * @returns The new recovery code, to show to the user exactly once.
 */
export async function regenerateRecoveryCode(currentCredential: Credential): Promise<string> {
  const response = await call<RecoveryCodeResponse>('regenerate_recovery_code', {
    currentCredential,
  });
  return response.recoveryCode;
}

/** Reads the app settings (available while locked). */
export function getSettings(): Promise<Settings> {
  return call('get_settings');
}

/** Sets the inactivity auto-lock timeout, in minutes. Requires an unlocked vault. */
export function setAutoLockTimeout(minutes: number): Promise<void> {
  return call('set_auto_lock_timeout', { minutes });
}
