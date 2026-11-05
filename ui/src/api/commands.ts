/**
 * Typed wrappers over the Tauri commands: the UI's whole IPC surface.
 *
 * Each function calls one command declared in `src-tauri/build.rs` and
 * rejects with a {@link CommandError} on failure. Secrets passed here go
 * straight to Rust; the UI never receives key material, only the recovery
 * code at the two moments it must be shown.
 */
import { Channel, invoke, type InvokeArgs } from '@tauri-apps/api/core';

import { toCommandError, type CommandError } from './errors';
import type {
  Attachment,
  CopyResult,
  Credential,
  ItemField,
  ItemDetails,
  ItemInput,
  ItemSummary,
  ItemUpdate,
  RecoveryCodeResponse,
  RevealedValue,
  SecretItemField,
  Settings,
  TransferProgress,
  VaultStatus,
} from './types';

/**
 * Names of the commands this UI calls.
 *
 * Keep in sync with `COMMANDS` in `src-tauri/build.rs`: the shell's
 * capability allowlist is generated from that list, so a name missing
 * there is rejected at runtime.
 */
export type CommandName =
  | 'vault_status'
  | 'create_vault'
  | 'unlock'
  | 'lock'
  | 'change_master_password'
  | 'regenerate_recovery_code'
  | 'get_settings'
  | 'set_auto_lock_timeout'
  | 'set_clipboard_clear_delay'
  | 'list_items'
  | 'get_item'
  | 'reveal_item_field'
  | 'copy_item_field'
  | 'create_item'
  | 'update_item'
  | 'delete_item'
  | 'add_attachment'
  | 'export_attachment'
  | 'delete_attachment';

// The single path to `invoke`: every rejection is normalized here, so
// callers only ever see a `CommandError`. Nothing is logged, because the
// arguments may hold a password or recovery code.
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
 * @param password - The new master password. Sent to the shell once; this
 *   module keeps no copy.
 * @returns The recovery code, to show to the user exactly once.
 */
export async function createVault(password: string): Promise<string> {
  const response = await call<RecoveryCodeResponse>('create_vault', { password });
  return response.recoveryCode;
}

/**
 * Unlocks the vault with the master password or the recovery code.
 *
 * @param credential - The secret to try. Sent to the shell once; this
 *   module keeps no copy.
 */
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

/** Sets how long a copied field stays on the clipboard, in seconds. Requires an unlocked vault. */
export function setClipboardClearDelay(seconds: number): Promise<void> {
  return call('set_clipboard_clear_delay', { seconds });
}

/**
 * Copies one field of an item to the clipboard. The shell reads and copies
 * the value itself, so it never reaches the UI, and clears the clipboard
 * after the configured delay.
 *
 * @param id - The item's identifier.
 * @param field - Which field to copy; plain or secret.
 * @returns Seconds until the clipboard is cleared.
 */
export async function copyItemField(id: string, field: ItemField): Promise<number> {
  const response = await call<CopyResult>('copy_item_field', { id, field });
  return response.clearAfterSeconds;
}

/** Lists the items of the unlocked vault, without secret fields. */
export function listItems(): Promise<ItemSummary[]> {
  return call('list_items');
}

/**
 * Reads one item's plain fields; secret values are not included.
 *
 * @param id - The item's identifier.
 */
export function getItem(id: string): Promise<ItemDetails> {
  return call('get_item', { id });
}

/**
 * Reads one secret field of an item. Call it only when the user asks to
 * see the value, and drop the result when it is hidden again.
 *
 * @param id - The item's identifier.
 * @param field - Which secret field to read.
 */
export async function revealItemField(id: string, field: SecretItemField): Promise<string> {
  const response = await call<RevealedValue>('reveal_item_field', { id, field });
  return response.value;
}

/**
 * Adds an item.
 *
 * @returns The new item's identifier.
 */
export function createItem(item: ItemInput): Promise<string> {
  return call('create_item', { item });
}

/**
 * Updates an item; secret fields set to `null` keep their stored values.
 *
 * @param id - The item's identifier.
 * @param item - The new content; its kind must match the stored item's.
 */
export function updateItem(id: string, item: ItemUpdate): Promise<void> {
  return call('update_item', { id, item });
}

/**
 * Deletes an item and its attachments.
 *
 * @param id - The item's identifier.
 */
export function deleteItem(id: string): Promise<void> {
  return call('delete_item', { id });
}

/**
 * Lets the user pick a file in a native dialog, shown by the shell, and
 * attaches it to an item. The UI never sees the file's path or contents.
 *
 * @param itemId - The item to attach the file to.
 * @param onProgress - Called with the encryption progress while it runs.
 * @returns The new attachment, or `null` if the user closed the dialog.
 */
export function addAttachment(
  itemId: string,
  onProgress: (progress: TransferProgress) => void,
): Promise<Attachment | null> {
  return call('add_attachment', { itemId, onProgress: new Channel(onProgress) });
}

/**
 * Lets the user choose where to save an attachment in a native dialog,
 * shown by the shell, and decrypts it there.
 *
 * @param itemId - The item the attachment belongs to.
 * @param attachmentId - The attachment to export.
 * @param onProgress - Called with the decryption progress while it runs.
 * @returns `false` if the user closed the dialog.
 */
export function exportAttachment(
  itemId: string,
  attachmentId: string,
  onProgress: (progress: TransferProgress) => void,
): Promise<boolean> {
  return call('export_attachment', {
    itemId,
    attachmentId,
    onProgress: new Channel(onProgress),
  });
}

/**
 * Removes an attachment from an item and deletes its encrypted data.
 *
 * @param itemId - The item the attachment belongs to.
 * @param attachmentId - The attachment to remove.
 */
export function deleteAttachment(itemId: string, attachmentId: string): Promise<void> {
  return call('delete_attachment', { itemId, attachmentId });
}
