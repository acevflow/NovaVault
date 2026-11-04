import { useRef, useState, type SubmitEvent } from 'react';

import { createItem, revealItemField, updateItem } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import type {
  ItemDetails,
  ItemInput,
  ItemKind,
  ItemUpdate,
  SecretItemField,
} from '../../api/types';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { PasswordField } from '../../components/PasswordField';
import { ScreenHeading } from '../../components/ScreenHeading';
import { TextAreaField } from '../../components/TextAreaField';
import { TextField } from '../../components/TextField';
import { KIND_LABELS } from './kinds';

/** Props of {@link ItemForm}. */
export interface ItemFormProps {
  /** The kind of item to create, or of the item being edited. */
  readonly kind: ItemKind;
  /** The item being edited, or `null` to create a new one. */
  readonly existing: ItemDetails | null;
  /** Called with the identifier of the saved item. */
  readonly onSaved: (id: string) => void;
  /** Leaves the form without saving. */
  readonly onCancel: () => void;
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

/**
 * The form's copy of one secret field. When editing, secret fields start
 * empty and not `loaded`: an empty, unloaded field is sent as `null`, so the
 * shell keeps the stored value without the UI ever seeing it. Revealing the
 * current value loads it; after that, emptying the field clears it.
 */
interface SecretDraft {
  readonly value: string;
  readonly loaded: boolean;
}

/** The secret fields of each kind, with their labels. */
const SECRET_FIELDS: Record<
  ItemKind,
  readonly { readonly field: SecretItemField; readonly label: string }[]
> = {
  password: [
    { field: 'password', label: 'Password' },
    { field: 'notes', label: 'Notes' },
  ],
  note: [{ field: 'body', label: 'Text' }],
  secret: [
    { field: 'value', label: 'Value' },
    { field: 'notes', label: 'Notes' },
  ],
};

/** Whether the existing item has a value in `field`. */
function hasStored(existing: ItemDetails, field: SecretItemField): boolean {
  switch (existing.kind) {
    case 'password':
      return field === 'password' ? existing.hasPassword : existing.hasNotes;
    case 'note':
      return existing.hasBody;
    case 'secret':
      return field === 'value' ? existing.hasValue : existing.hasNotes;
  }
}

function initialSecrets(isNew: boolean): Record<SecretItemField, SecretDraft> {
  const draft = { value: '', loaded: isNew };
  return { password: draft, notes: draft, body: draft, value: draft };
}

/** Creates an item of a given kind, or edits an existing one. */
export function ItemForm({ kind, existing, onSaved, onCancel, onLocked }: ItemFormProps) {
  const isNew = existing === null;
  const [title, setTitle] = useState(existing?.title ?? '');
  const [username, setUsername] = useState(existing?.kind === 'password' ? existing.username : '');
  const [url, setUrl] = useState(existing?.kind === 'password' ? existing.url : '');
  const [secrets, setSecrets] = useState(() => initialSecrets(isNew));
  const [titleError, setTitleError] = useState<string | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const titleRef = useRef<HTMLInputElement>(null);

  function setSecret(field: SecretItemField, draft: SecretDraft) {
    setSecrets((current) => ({ ...current, [field]: draft }));
  }

  /** What to send for `field`: its text, or `null` to keep the stored value. */
  function secretOut(field: SecretItemField): string | null {
    const draft = secrets[field];
    return draft.loaded || draft.value !== '' ? draft.value : null;
  }

  function report(reason: unknown) {
    if (isCommandError(reason, 'locked')) {
      onLocked();
      return;
    }
    setError(errorMessage(reason));
  }

  async function revealCurrent(field: SecretItemField) {
    if (existing === null) return;
    setError(null);
    try {
      setSecret(field, { value: await revealItemField(existing.id, field), loaded: true });
    } catch (reason) {
      report(reason);
    }
  }

  function buildInput(): ItemInput {
    const text = (field: SecretItemField) => secrets[field].value;
    switch (kind) {
      case 'password':
        return { kind, title, username, url, password: text('password'), notes: text('notes') };
      case 'note':
        return { kind, title, body: text('body') };
      case 'secret':
        return { kind, title, value: text('value'), notes: text('notes') };
    }
  }

  function buildUpdate(): ItemUpdate {
    switch (kind) {
      case 'password':
        return {
          kind,
          title,
          username,
          url,
          password: secretOut('password'),
          notes: secretOut('notes'),
        };
      case 'note':
        return { kind, title, body: secretOut('body') };
      case 'secret':
        return { kind, title, value: secretOut('value'), notes: secretOut('notes') };
    }
  }

  async function save() {
    setBusy(true);
    try {
      if (existing === null) {
        onSaved(await createItem(buildInput()));
      } else {
        await updateItem(existing.id, buildUpdate());
        onSaved(existing.id);
      }
    } catch (reason) {
      report(reason);
      setBusy(false);
    }
  }

  function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (title.trim() === '') {
      setTitleError('Enter a title.');
      titleRef.current?.focus();
      return;
    }
    setTitleError(undefined);
    void save();
  }

  const heading = isNew
    ? `New ${KIND_LABELS[kind].toLowerCase()}`
    : `Edit “${existing.title || 'Untitled'}”`;

  return (
    <section className="card" aria-label={heading}>
      <ScreenHeading level={2}>{heading}</ScreenHeading>
      <form className="stack" noValidate onSubmit={handleSubmit}>
        <TextField
          ref={titleRef}
          label="Title"
          value={title}
          required
          autoComplete="off"
          error={titleError}
          onChange={(event) => {
            setTitle(event.target.value);
          }}
        />
        {kind === 'password' ? (
          <>
            <TextField
              label="Username"
              value={username}
              autoComplete="off"
              spellCheck={false}
              autoCapitalize="off"
              onChange={(event) => {
                setUsername(event.target.value);
              }}
            />
            <TextField
              label="Website"
              type="url"
              value={url}
              autoComplete="off"
              spellCheck={false}
              placeholder="https://"
              onChange={(event) => {
                setUrl(event.target.value);
              }}
            />
          </>
        ) : null}
        {SECRET_FIELDS[kind].map(({ field, label }) => (
          <SecretInput
            key={field}
            field={field}
            label={label}
            draft={secrets[field]}
            canReveal={existing !== null && hasStored(existing, field)}
            onChange={(value) => {
              setSecret(field, { value, loaded: secrets[field].loaded });
            }}
            onReveal={() => void revealCurrent(field)}
          />
        ))}
        {error === null ? null : <Alert tone="error">{error}</Alert>}
        <div className="actions">
          <Button type="submit" variant="primary" busy={busy}>
            {busy ? 'Saving…' : 'Save'}
          </Button>
          <Button disabled={busy} onClick={onCancel}>
            Cancel
          </Button>
        </div>
      </form>
    </section>
  );
}

interface SecretInputProps {
  readonly field: SecretItemField;
  readonly label: string;
  readonly draft: SecretDraft;
  /** Editing an item that has a stored value the user may load. */
  readonly canReveal: boolean;
  readonly onChange: (value: string) => void;
  readonly onReveal: () => void;
}

/** The input for one secret field, with the keep-or-reveal choice when editing. */
function SecretInput({ field, label, draft, canReveal, onChange, onReveal }: SecretInputProps) {
  const keeping = canReveal && !draft.loaded;
  const lower = label.toLowerCase();
  const description = keeping ? `Leave empty to keep the current ${lower}.` : undefined;
  const reveal = keeping ? (
    <div>
      <Button className="button--small" onClick={onReveal}>
        Reveal current {lower}
      </Button>
    </div>
  ) : null;

  if (field === 'notes' || field === 'body') {
    return (
      <TextAreaField
        label={label}
        value={draft.value}
        rows={field === 'body' ? 8 : 4}
        description={description}
        trailing={reveal}
        onChange={(event) => {
          onChange(event.target.value);
        }}
      />
    );
  }
  return (
    <div className="stack-tight">
      <PasswordField
        label={label}
        value={draft.value}
        autoComplete="off"
        description={description}
        onChange={(event) => {
          onChange(event.target.value);
        }}
      />
      {reveal}
    </div>
  );
}
