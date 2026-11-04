import { useEffect, useState } from 'react';

import { deleteItem, getItem } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import type { ItemDetails as Details } from '../../api/types';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { CopyButton } from '../../components/CopyButton';
import { ScreenHeading } from '../../components/ScreenHeading';
import { SecretField } from '../../components/SecretField';
import { ConfirmDelete } from './ConfirmDelete';
import { KIND_LABELS } from './kinds';

/** Props of {@link ItemDetails}. */
export interface ItemDetailsProps {
  /** The item to show. Remount (with a `key`) to show another one. */
  readonly itemId: string;
  /** Opens the edit form with the item's plain fields. */
  readonly onEdit: (details: Details) => void;
  /** Called after the item was deleted. */
  readonly onDeleted: () => void;
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

/** One item's fields, with secret fields hidden until revealed, and its actions. */
export function ItemDetails({ itemId, onEdit, onDeleted, onLocked }: ItemDetailsProps) {
  const [details, setDetails] = useState<Details | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    let active = true;
    getItem(itemId)
      .then((loaded) => {
        if (active) setDetails(loaded);
      })
      .catch((reason: unknown) => {
        if (!active) return;
        if (isCommandError(reason, 'locked')) {
          onLocked();
          return;
        }
        setError(errorMessage(reason));
      });
    return () => {
      active = false;
    };
  }, [itemId, onLocked]);

  async function remove() {
    setDeleting(true);
    setError(null);
    try {
      await deleteItem(itemId);
      onDeleted();
    } catch (reason) {
      if (isCommandError(reason, 'locked')) {
        onLocked();
        return;
      }
      setError(errorMessage(reason));
      setDeleting(false);
      setConfirming(false);
    }
  }

  if (details === null) {
    return (
      <section className="card" aria-label="Item details">
        {error === null ? <p role="status">Loading item…</p> : <Alert tone="error">{error}</Alert>}
      </section>
    );
  }

  return (
    <section className="card" aria-label="Item details">
      <p className="badge">{KIND_LABELS[details.kind]}</p>
      <ScreenHeading level={2}>{details.title || 'Untitled'}</ScreenHeading>
      <dl className="details">
        <Fields details={details} onLocked={onLocked} />
        <div className="details__row">
          <dt>Last updated</dt>
          <dd>{formatTime(details.updatedAtMs)}</dd>
        </div>
      </dl>
      {error === null ? null : <Alert tone="error">{error}</Alert>}
      {confirming ? (
        <ConfirmDelete
          title={details.title}
          busy={deleting}
          onConfirm={() => void remove()}
          onCancel={() => {
            setConfirming(false);
          }}
        />
      ) : (
        <div className="actions">
          <Button
            variant="primary"
            onClick={() => {
              onEdit(details);
            }}
          >
            Edit
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              setConfirming(true);
            }}
          >
            Delete
          </Button>
        </div>
      )}
    </section>
  );
}

interface FieldsProps {
  readonly details: Details;
  readonly onLocked: () => void;
}

/** The kind-specific rows of the description list. */
function Fields({ details, onLocked }: FieldsProps) {
  const common = { itemId: details.id, onLocked };
  switch (details.kind) {
    case 'password':
      return (
        <>
          <PlainField {...common} label="Username" field="username" value={details.username} />
          <SecretField
            {...common}
            label="Password"
            field="password"
            hasValue={details.hasPassword}
          />
          <PlainField {...common} label="Website" field="url" value={details.url} />
          <SecretField
            {...common}
            label="Notes"
            field="notes"
            hasValue={details.hasNotes}
            multiline
          />
        </>
      );
    case 'note':
      return (
        <SecretField {...common} label="Text" field="body" hasValue={details.hasBody} multiline />
      );
    case 'secret':
      return (
        <>
          <SecretField {...common} label="Value" field="value" hasValue={details.hasValue} />
          <SecretField
            {...common}
            label="Notes"
            field="notes"
            hasValue={details.hasNotes}
            multiline
          />
        </>
      );
  }
}

interface PlainFieldProps {
  readonly itemId: string;
  readonly field: 'username' | 'url';
  readonly label: string;
  readonly value: string;
  readonly onLocked: () => void;
}

/** A non-secret field, always visible, with a copy button. */
function PlainField({ itemId, field, label, value, onLocked }: PlainFieldProps) {
  return (
    <div className="details__row">
      <dt>{label}</dt>
      <dd className="secret-field">
        {value === '' ? (
          <span className="muted">Empty</span>
        ) : (
          <>
            <span className="secret-field__value secret-field__value--plain">{value}</span>
            <span className="secret-field__actions">
              <CopyButton itemId={itemId} field={field} label={label} onLocked={onLocked} />
            </span>
          </>
        )}
      </dd>
    </div>
  );
}
