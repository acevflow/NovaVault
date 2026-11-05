import { useId, useState } from 'react';

import { addAttachment, deleteAttachment, exportAttachment } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import type { Attachment, TransferProgress } from '../../api/types';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { formatBytes, percentDone } from '../../format/bytes';
import { formatDateTime } from '../../format/dateTime';
import { ConfirmDelete } from './ConfirmDelete';

/** Props of {@link AttachmentList}. */
export interface AttachmentListProps {
  /** The item the attachments belong to. */
  readonly itemId: string;
  /** The item's attachments. */
  readonly attachments: readonly Attachment[];
  /** Called with the new list after a file was attached or removed. */
  readonly onChange: (attachments: readonly Attachment[]) => void;
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

/** A transfer in progress: what is happening and how far it is. */
interface Transfer {
  /** Adding a file, or exporting one. */
  readonly kind: 'attach' | 'export';
  /** Says what is happening, for example "Attaching file". */
  readonly label: string;
  /** `null` while the native dialog is open, before any data moves. */
  readonly progress: TransferProgress | null;
}

/**
 * An item's attached files, with actions to attach, export and remove
 * them.
 *
 * Files are picked and saved in native dialogs that the shell opens, and
 * encrypted or decrypted in Rust: this component only learns names, sizes
 * and progress. One transfer runs at a time; the other actions are
 * disabled meanwhile.
 */
export function AttachmentList({ itemId, attachments, onChange, onLocked }: AttachmentListProps) {
  const headingId = useId();
  const [transfer, setTransfer] = useState<Transfer | null>(null);
  const [confirming, setConfirming] = useState<Attachment | null>(null);
  const [removing, setRemoving] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const busy = transfer !== null || removing;

  function start(kind: Transfer['kind'], label: string) {
    setNotice(null);
    setError(null);
    setTransfer({ kind, label, progress: null });
  }

  function onProgress(progress: TransferProgress) {
    setTransfer((current) => (current === null ? null : { ...current, progress }));
  }

  function fail(reason: unknown) {
    if (isCommandError(reason, 'locked')) {
      onLocked();
      return;
    }
    setError(errorMessage(reason));
  }

  async function attach() {
    start('attach', 'Attaching file');
    try {
      const added = await addAttachment(itemId, onProgress);
      // `null`: the user closed the dialog, which needs no message.
      if (added !== null) {
        onChange([...attachments, added]);
        setNotice(`Attached “${added.name}”.`);
      }
    } catch (reason) {
      fail(reason);
    } finally {
      setTransfer(null);
    }
  }

  async function exportFile(attachment: Attachment) {
    start('export', `Exporting “${attachment.name}”`);
    try {
      if (await exportAttachment(itemId, attachment.id, onProgress)) {
        setNotice(`Exported “${attachment.name}”.`);
      }
    } catch (reason) {
      fail(reason);
    } finally {
      setTransfer(null);
    }
  }

  async function remove(attachment: Attachment) {
    setRemoving(true);
    setNotice(null);
    setError(null);
    try {
      await deleteAttachment(itemId, attachment.id);
      onChange(attachments.filter((candidate) => candidate.id !== attachment.id));
      setNotice(`Removed “${attachment.name}”.`);
      setConfirming(null);
    } catch (reason) {
      fail(reason);
      setConfirming(null);
    } finally {
      setRemoving(false);
    }
  }

  return (
    <section className="attachments" aria-labelledby={headingId}>
      <h3 id={headingId} className="attachments__heading">
        Attachments
      </h3>
      {attachments.length === 0 ? (
        <p className="muted">No files attached.</p>
      ) : (
        <ul className="attachment-list">
          {attachments.map((attachment) => (
            <li key={attachment.id} className="attachment-list__item">
              <span className="attachment-list__name">{attachment.name}</span>
              <span className="attachment-list__meta muted">
                {formatBytes(attachment.size)} · added {formatDateTime(attachment.addedAtMs)}
              </span>
              <span className="attachment-list__actions">
                <Button
                  className="button--small"
                  aria-label={`Export ${attachment.name}`}
                  disabled={busy}
                  onClick={() => void exportFile(attachment)}
                >
                  Export
                </Button>
                <Button
                  className="button--small"
                  aria-label={`Remove ${attachment.name}`}
                  disabled={busy}
                  onClick={() => {
                    setConfirming(attachment);
                  }}
                >
                  Remove
                </Button>
              </span>
            </li>
          ))}
        </ul>
      )}
      {transfer === null ? null : <TransferStatus transfer={transfer} />}
      {notice === null ? null : <Alert tone="success">{notice}</Alert>}
      {error === null ? null : <Alert tone="error">{error}</Alert>}
      {confirming === null ? (
        <div className="actions">
          <Button busy={transfer?.kind === 'attach'} disabled={busy} onClick={() => void attach()}>
            Attach file
          </Button>
        </div>
      ) : (
        <ConfirmDelete
          subject="attachment"
          title={confirming.name}
          busy={removing}
          onConfirm={() => void remove(confirming)}
          onCancel={() => {
            setConfirming(null);
          }}
        />
      )}
    </section>
  );
}

/** The progress bar of a running transfer. */
function TransferStatus({ transfer }: { readonly transfer: Transfer }) {
  const labelId = useId();
  const { progress } = transfer;
  return (
    <div className="transfer">
      <span id={labelId} className="transfer__label">
        {transfer.label}
      </span>
      {progress === null ? (
        <>
          {/* No value: an indeterminate bar while the dialog is open. */}
          <progress className="transfer__bar" aria-labelledby={labelId} />
          <span className="transfer__detail muted">
            {transfer.kind === 'attach'
              ? 'Choose a file in the dialog…'
              : 'Choose where to save it in the dialog…'}
          </span>
        </>
      ) : (
        <>
          <progress
            className="transfer__bar"
            aria-labelledby={labelId}
            max={100}
            value={percentDone(progress.done, progress.total)}
          />
          <span className="transfer__detail muted">
            {percentDone(progress.done, progress.total)}% · {formatBytes(progress.done)} of{' '}
            {formatBytes(progress.total)}
          </span>
        </>
      )}
    </div>
  );
}
