import { useEffect, useId, useRef } from 'react';

import { Button } from '../../components/Button';

/** Props of {@link ConfirmDelete}. */
export interface ConfirmDeleteProps {
  /** Title of the item to delete, quoted in the question. */
  readonly title: string;
  /** Whether the deletion is in progress. */
  readonly busy: boolean;
  /** Deletes the item. */
  readonly onConfirm: () => void;
  /** Keeps the item. */
  readonly onCancel: () => void;
}

/**
 * Asks the user to confirm a deletion, inline below the item.
 *
 * Focus starts on Cancel, so pressing Enter by reflex keeps the item, and
 * Escape cancels.
 */
export function ConfirmDelete({ title, busy, onConfirm, onCancel }: ConfirmDeleteProps) {
  const headingId = useId();
  const messageId = useId();
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancelRef.current?.focus();
  }, []);

  // Listened for on the document so Escape works wherever focus is.
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape' && !busy) {
        onCancel();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [busy, onCancel]);

  return (
    <div
      role="alertdialog"
      aria-labelledby={headingId}
      aria-describedby={messageId}
      className="confirm"
    >
      <h3 id={headingId} className="confirm__heading">
        Delete this item?
      </h3>
      <p id={messageId}>
        “{title || 'Untitled'}” will be deleted from the vault. This cannot be undone.
      </p>
      <div className="actions">
        <Button variant="danger" busy={busy} onClick={onConfirm}>
          {busy ? 'Deleting…' : 'Delete'}
        </Button>
        <Button ref={cancelRef} disabled={busy} onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
