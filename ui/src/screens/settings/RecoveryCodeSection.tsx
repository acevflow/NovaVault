import { useId, useRef, useState, type SubmitEvent } from 'react';

import { regenerateRecoveryCode } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { RecoveryCodeFlow } from '../../recovery/RecoveryCodeFlow';
import { CurrentCredentialFields } from './CurrentCredentialFields';
import { useCredential } from './useCredential';

/** Props of {@link RecoveryCodeSection}. */
export interface RecoveryCodeSectionProps {
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

/**
 * Regenerates the recovery code after re-authentication, then walks the
 * user through saving and confirming the new one.
 */
export function RecoveryCodeSection({ onLocked }: RecoveryCodeSectionProps) {
  const headingId = useId();
  const current = useCredential();
  const [newCode, setNewCode] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const currentInput = useRef<HTMLInputElement>(null);

  async function regenerate() {
    setBusy(true);
    try {
      const code = await regenerateRecoveryCode(current.credential);
      current.clear();
      setNewCode(code);
    } catch (reason) {
      if (isCommandError(reason, 'locked')) {
        onLocked();
        return;
      }
      current.clear();
      setError(
        isCommandError(reason, 'invalid_credentials')
          ? 'The current password or recovery code is not correct. Your recovery code was not changed.'
          : errorMessage(reason),
      );
      currentInput.current?.focus();
    } finally {
      setBusy(false);
    }
  }

  function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setDone(false);
    setError(null);
    if (current.secret === '') {
      setError('Enter your current master password or recovery code.');
      currentInput.current?.focus();
      return;
    }
    void regenerate();
  }

  return (
    <section className="card" aria-labelledby={headingId}>
      <h2 id={headingId}>Recovery code</h2>
      {newCode === null ? (
        <form className="stack" onSubmit={handleSubmit} noValidate>
          <p className="muted">Generate a new code if yours was lost or seen by someone else.</p>
          <Alert tone="warning">
            Your current recovery code stops working as soon as the new one is generated.
          </Alert>
          <CurrentCredentialFields
            method={current.method}
            secret={current.secret}
            onMethodChange={current.setMethod}
            onSecretChange={current.setSecret}
            disabled={busy}
            inputRef={currentInput}
          />
          {error === null ? null : <Alert tone="error">{error}</Alert>}
          {done ? (
            <Alert tone="success">Your new recovery code is saved and confirmed.</Alert>
          ) : null}
          <div className="actions">
            <Button type="submit" variant="danger" busy={busy}>
              {busy ? 'Generating…' : 'Generate new recovery code'}
            </Button>
          </div>
        </form>
      ) : (
        <RecoveryCodeFlow
          code={newCode}
          headingLevel={3}
          intro="This new code replaces your previous one, which no longer works."
          onDone={() => {
            // Drop the code from memory once it is confirmed.
            setNewCode(null);
            setDone(true);
          }}
        />
      )}
    </section>
  );
}
