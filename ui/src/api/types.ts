/**
 * Types exchanged with the Rust shell over Tauri IPC.
 *
 * They mirror `src-tauri/src/dto.rs` and `src-tauri/src/settings.rs`; keep
 * both sides in sync. Field names are camelCase, as serialized by serde.
 */

/** Whether a vault exists and whether it is unlocked. */
export interface VaultStatus {
  /** A vault file exists at the app's vault location. */
  readonly exists: boolean;
  /** The vault is currently unlocked. */
  readonly unlocked: boolean;
}

/** How a credential proves who the user is. */
export type CredentialMethod = 'password' | 'recoveryCode';

/**
 * A secret that unlocks the vault or authorizes a key slot change: the
 * master password or the emergency recovery code.
 */
export interface Credential {
  /** Which secret {@link Credential.secret} is. */
  readonly method: CredentialMethod;
  /** The master password, or the recovery code in any case, with or without dashes. */
  readonly secret: string;
}

/** A newly created or regenerated recovery code, shown to the user once. */
export interface RecoveryCodeResponse {
  /** The code in display form: `XXXX-XXXX-…`, 8 groups of 4. */
  readonly recoveryCode: string;
}

/** App settings stored outside the vault. Not secret. */
export interface Settings {
  /** Minutes without activity before the vault locks itself. */
  readonly autoLockMinutes: number;
}
