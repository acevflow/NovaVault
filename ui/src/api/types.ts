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
  /** Seconds before a copied field is cleared from the clipboard. */
  readonly clipboardClearSeconds: number;
}

/** The type of a vault item. */
export type ItemKind = 'password' | 'note' | 'secret';

/** One entry of the item list. Never contains secret fields. */
export interface ItemSummary {
  /** Item identifier, 32 lower-case hexadecimal digits. */
  readonly id: string;
  /** The item's type. */
  readonly kind: ItemKind;
  /** Display name. */
  readonly title: string;
  /** Creation time, milliseconds since the Unix epoch (UTC). */
  readonly createdAtMs: number;
  /** Last update time, milliseconds since the Unix epoch (UTC). */
  readonly updatedAtMs: number;
}

/** One file attached to an item. The file's contents never reach the UI. */
export interface Attachment {
  /** Attachment identifier, 32 lower-case hexadecimal digits. */
  readonly id: string;
  /** The original file name. */
  readonly name: string;
  /** Size in bytes. */
  readonly size: number;
  /** When the file was attached, milliseconds since the Unix epoch (UTC). */
  readonly addedAtMs: number;
}

/** Progress of an attachment being added or exported. */
export interface TransferProgress {
  /** Bytes encrypted or decrypted so far. */
  readonly done: number;
  /** Bytes in total. */
  readonly total: number;
}

/** Fields every item's details share. */
interface ItemDetailsBase {
  /** Item identifier. */
  readonly id: string;
  /** Display name. */
  readonly title: string;
  /** Creation time, milliseconds since the Unix epoch (UTC). */
  readonly createdAtMs: number;
  /** Last update time, milliseconds since the Unix epoch (UTC). */
  readonly updatedAtMs: number;
  /** The item's attachments, in the order they were added. */
  readonly attachments: readonly Attachment[];
}

/**
 * One item as shown on its details screen: plain fields, and for each
 * secret field only whether it is set. Secret values come one at a time
 * from `revealItemField`.
 */
export type ItemDetails =
  | (ItemDetailsBase & {
      readonly kind: 'password';
      readonly username: string;
      readonly url: string;
      readonly hasPassword: boolean;
      readonly hasNotes: boolean;
    })
  | (ItemDetailsBase & { readonly kind: 'note'; readonly hasBody: boolean })
  | (ItemDetailsBase & {
      readonly kind: 'secret';
      readonly hasValue: boolean;
      readonly hasNotes: boolean;
    });

/** The content of a new item, tagged by `kind`. */
export type ItemInput =
  | {
      readonly kind: 'password';
      readonly title: string;
      readonly username: string;
      readonly password: string;
      readonly url: string;
      readonly notes: string;
    }
  | { readonly kind: 'note'; readonly title: string; readonly body: string }
  | {
      readonly kind: 'secret';
      readonly title: string;
      readonly value: string;
      readonly notes: string;
    };

/**
 * New content for an existing item. A secret field set to `null` keeps its
 * stored value. The kind must match the stored item's kind.
 */
export type ItemUpdate =
  | {
      readonly kind: 'password';
      readonly title: string;
      readonly username: string;
      readonly password: string | null;
      readonly url: string;
      readonly notes: string | null;
    }
  | { readonly kind: 'note'; readonly title: string; readonly body: string | null }
  | {
      readonly kind: 'secret';
      readonly title: string;
      readonly value: string | null;
      readonly notes: string | null;
    };

/** A field of an item that can be revealed or copied. */
export type ItemField = 'username' | 'password' | 'url' | 'notes' | 'body' | 'value';

/** A secret field: hidden until the user reveals it. */
export type SecretItemField = Exclude<ItemField, 'username' | 'url'>;

/** The outcome of copying a field; the value itself never comes back. */
export interface CopyResult {
  /** Seconds until the shell clears the clipboard. */
  readonly clearAfterSeconds: number;
}

/** A revealed secret field. */
export interface RevealedValue {
  /** The field's value. */
  readonly value: string;
}
