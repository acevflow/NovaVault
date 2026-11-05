import { useEffect, useId, useRef } from 'react';

import { Button } from '../../components/Button';

/** What {@link ConfirmDelete} deletes. */
export type DeleteSubject = 'item' | 'attachment';

const WORDING: Record<
  DeleteSubject,
  { heading: string; consequence: string; action: string; busy: string }
> = {
  item: {
    heading: 'Delete this item?',
    consequence: 'will be deleted from the vault',
    action: 'Delete',
    busy: 'Deleting…',
  },
  attachment: {
    heading: 'Remove this attachment?',
    consequence: 'will be removed from this item and its encrypted copy deleted',
    action: 'Remove',
    busy: 'Removing…',
  },
};

/** Props of {@link ConfirmDelete}. */
export interface ConfirmDeleteProps {
  /** Title of the item (or name of the file) to delete, quoted in the question. */
  readonly title: string;
  /** What is deleted; `item` by default. */
  readonly subject?: DeleteSubject;
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
export function ConfirmDelete({
  title,
  subject = 'item',
  busy,
  onConfirm,
  onCancel,
}: ConfirmDeleteProps) {
  const wording = WORDING[subject];
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
        {wording.heading}
      </h3>
      <p id={messageId}>
        “{title || 'Untitled'}” {wording.consequence}. This cannot be undone.
      </p>
      <div className="actions">
        <Button variant="danger" busy={busy} onClick={onConfirm}>
          {busy ? wording.busy : wording.action}
        </Button>
        <Button ref={cancelRef} disabled={busy} onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
