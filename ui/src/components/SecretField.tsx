import { useState } from 'react';

import { revealItemField } from '../api/commands';
import { errorMessage, isCommandError } from '../api/errors';
import type { SecretItemField } from '../api/types';
import { CopyButton } from './CopyButton';

/** Props of {@link SecretField}. */
export interface SecretFieldProps {
  /** Visible label, for example "Password". */
  readonly label: string;
  /** The item the field belongs to. */
  readonly itemId: string;
  /** Which secret field to reveal. */
  readonly field: SecretItemField;
  /** Whether the field has a value; an empty field offers no reveal. */
  readonly hasValue: boolean;
  /** Keeps line breaks when revealed (notes, note text). */
  readonly multiline?: boolean;
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

/**
 * One secret field of an item, as a row of a description list. Hidden by
 * default: the value is fetched from the shell only when the user asks to
 * reveal it, kept only in this component's state, and dropped as soon as
 * it is hidden again or the component unmounts.
 */
export function SecretField({
  label,
  itemId,
  field,
  hasValue,
  multiline = false,
  onLocked,
}: SecretFieldProps) {
  const [value, setValue] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function reveal() {
    setBusy(true);
    setError(null);
    try {
      setValue(await revealItemField(itemId, field));
    } catch (reason) {
      if (isCommandError(reason, 'locked')) {
        onLocked();
        return;
      }
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  function toggle() {
    if (value === null) {
      void reveal();
    } else {
      setValue(null);
    }
  }

  return (
    <div className="details__row">
      <dt>{label}</dt>
      <dd className="secret-field">
        {!hasValue ? (
          <span className="muted">Empty</span>
        ) : (
          <>
            {value === null ? (
              <span className="secret-field__mask">
                <span aria-hidden="true">••••••••</span>
                <span className="visually-hidden">Hidden</span>
              </span>
            ) : (
              <span
                className={
                  multiline
                    ? 'secret-field__value secret-field__value--multiline'
                    : 'secret-field__value'
                }
              >
                {value}
              </span>
            )}
            <span className="secret-field__actions">
              <button
                type="button"
                className="field__toggle"
                aria-label={`Reveal ${label.toLowerCase()}`}
                aria-pressed={value !== null}
                aria-busy={busy || undefined}
                disabled={busy}
                onClick={toggle}
              >
                {value === null ? 'Reveal' : 'Hide'}
              </button>
              <CopyButton itemId={itemId} field={field} label={label} onLocked={onLocked} />
            </span>
          </>
        )}
        {error === null ? null : (
          <span role="alert" className="field__error">
            {error}
          </span>
        )}
      </dd>
    </div>
  );
}
