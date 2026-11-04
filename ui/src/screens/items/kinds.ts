/**
 * Display names of item kinds.
 */
import type { ItemKind } from '../../api/types';

/** Every item kind, in the order the UI offers them. */
export const ITEM_KINDS: readonly ItemKind[] = ['password', 'note', 'secret'];

/** Singular label of each kind, for badges and headings. */
export const KIND_LABELS: Record<ItemKind, string> = {
  password: 'Password',
  note: 'Note',
  secret: 'Secret',
};

/** Plural label of each kind, for the type filter. */
export const KIND_PLURALS: Record<ItemKind, string> = {
  password: 'Passwords',
  note: 'Notes',
  secret: 'Secrets',
};
