import { useId, useRef, useState, type SubmitEvent } from 'react';

import { changeMasterPassword } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { newPasswordProblem, newPasswordProblemMessage } from '../../password/newPassword';
import { NewPasswordFields } from '../../password/NewPasswordFields';
import { usePasswordStrength } from '../../password/usePasswordStrength';
import { CurrentCredentialFields } from './CurrentCredentialFields';
import { useCredential } from './useCredential';

/** Props of {@link ChangePasswordSection}. */
export interface ChangePasswordSectionProps {
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

/** Changes the master password after re-authentication. The recovery code keeps working. */
export function ChangePasswordSection({ onLocked }: ChangePasswordSectionProps) {
  const headingId = useId();
  const current = useCredential();
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const currentInput = useRef<HTMLInputElement>(null);
  const estimate = usePasswordStrength(password);
  const problem = newPasswordProblem(password, confirmation, estimate);
  const problemMessage = newPasswordProblemMessage(problem);

  async function change() {
    setBusy(true);
    try {
      await changeMasterPassword(current.credential, password);
      current.clear();
      setPassword('');
      setConfirmation('');
      setSubmitted(false);
      setDone(true);
    } catch (reason) {
      if (isCommandError(reason, 'locked')) {
        onLocked();
        return;
      }
      current.clear();
      setError(
        isCommandError(reason, 'invalid_credentials')
          ? 'The current password or recovery code is not correct. Nothing was changed.'
          : errorMessage(reason),
      );
      currentInput.current?.focus();
    } finally {
      setBusy(false);
    }
  }

  function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitted(true);
    setDone(false);
    setError(null);
    if (current.secret === '') {
      setError('Enter your current master password or recovery code.');
      currentInput.current?.focus();
      return;
    }
    if (problem === null) {
      void change();
    }
  }

  return (
    <section className="card" aria-labelledby={headingId}>
      <h2 id={headingId}>Master password</h2>
      <p className="muted">
        Changing it re-encrypts only the vault key, so it is quick. Your recovery code keeps
        working.
      </p>
      <form className="stack" onSubmit={handleSubmit} noValidate>
        <CurrentCredentialFields
          method={current.method}
          secret={current.secret}
          onMethodChange={current.setMethod}
          onSecretChange={current.setSecret}
          disabled={busy}
          inputRef={currentInput}
        />
        <NewPasswordFields
          label="New master password"
          password={password}
          confirmation={confirmation}
          estimate={estimate}
          onPasswordChange={(value) => {
            setPassword(value);
            setDone(false);
          }}
          onConfirmationChange={setConfirmation}
          showErrors={submitted}
          disabled={busy}
        />
        {submitted && problemMessage !== null ? <Alert tone="error">{problemMessage}</Alert> : null}
        {error === null ? null : <Alert tone="error">{error}</Alert>}
        {done ? (
          <Alert tone="success">
            Master password changed. Use the new password next time you unlock.
          </Alert>
        ) : null}
        <div className="actions">
          <Button type="submit" variant="primary" busy={busy}>
            {busy ? 'Changing password…' : 'Change master password'}
          </Button>
        </div>
      </form>
    </section>
  );
}
