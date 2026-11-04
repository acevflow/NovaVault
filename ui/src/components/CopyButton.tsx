import { useState } from 'react';

import { copyItemField } from '../api/commands';
import { errorMessage, isCommandError } from '../api/errors';
import type { ItemField } from '../api/types';
import { formatSeconds } from '../format/duration';

/** Props of {@link CopyButton}. */
export interface CopyButtonProps {
  /** The item the field belongs to. */
  readonly itemId: string;
  /** Which field to copy. */
  readonly field: ItemField;
  /** The field's visible label, used in the accessible name. */
  readonly label: string;
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

/**
 * Copies an item field to the clipboard. The shell reads and copies the
 * value, so it is never sent to the UI, and clears it after a delay; the
 * outcome is announced politely.
 */
export function CopyButton({ itemId, field, label, onLocked }: CopyButtonProps) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [failed, setFailed] = useState(false);

  async function copy() {
    setBusy(true);
    try {
      const seconds = await copyItemField(itemId, field);
      setFailed(false);
      setMessage(`Copied. The clipboard clears in ${formatSeconds(seconds)}.`);
    } catch (reason) {
      if (isCommandError(reason, 'locked')) {
        onLocked();
        return;
      }
      setFailed(true);
      setMessage(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <button
        type="button"
        className="field__toggle"
        aria-label={`Copy ${label.toLowerCase()}`}
        aria-busy={busy || undefined}
        disabled={busy}
        onClick={() => void copy()}
      >
        Copy
      </button>
      {/* Always rendered so screen readers announce changes to it. */}
      <span role="status" className={failed ? 'copy-status field__error' : 'copy-status muted'}>
        {message}
      </span>
    </>
  );
}
