import { useState, type SubmitEvent } from 'react';

import { createVault } from '../../api/commands';
import { errorMessage } from '../../api/errors';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { ScreenHeading } from '../../components/ScreenHeading';
import { newPasswordProblem, newPasswordProblemMessage } from '../../password/newPassword';
import { NewPasswordFields } from '../../password/NewPasswordFields';
import { usePasswordStrength } from '../../password/usePasswordStrength';
import { RecoveryCodeFlow } from '../../recovery/RecoveryCodeFlow';

/** Props of {@link CreateVaultScreen}. */
export interface CreateVaultScreenProps {
  /** Called when the vault exists and the user confirmed the recovery code. */
  readonly onCreated: () => void;
}

/**
 * First-run flow: choose a master password, then save and confirm the
 * one-time recovery code.
 *
 * The vault is created (and unlocked) as soon as the password is accepted,
 * because that is when the shell generates the recovery code.
 */
export function CreateVaultScreen({ onCreated }: CreateVaultScreenProps) {
  const [recoveryCode, setRecoveryCode] = useState<string | null>(null);

  if (recoveryCode !== null) {
    return (
      <section className="screen">
        <RecoveryCodeFlow
          code={recoveryCode}
          onDone={() => {
            // Drop the code from memory before leaving the flow.
            setRecoveryCode(null);
            onCreated();
          }}
        />
      </section>
    );
  }
  return <PasswordStep onCreated={setRecoveryCode} />;
}

interface PasswordStepProps {
  readonly onCreated: (recoveryCode: string) => void;
}

function PasswordStep({ onCreated }: PasswordStepProps) {
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const estimate = usePasswordStrength(password);
  const problem = newPasswordProblem(password, confirmation, estimate);
  const problemMessage = newPasswordProblemMessage(problem);

  async function create() {
    setBusy(true);
    setError(null);
    try {
      const code = await createVault(password);
      setPassword('');
      setConfirmation('');
      onCreated(code);
    } catch (reason) {
      setError(errorMessage(reason));
      setBusy(false);
    }
  }

  function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitted(true);
    if (problem === null) {
      void create();
    }
  }

  return (
    <section className="screen screen--narrow">
      <ScreenHeading>Create your vault</ScreenHeading>
      <p>
        Your master password encrypts everything in NovaVault. It never leaves this device and
        cannot be reset by anyone, so choose one you will remember.
      </p>
      <form className="stack" onSubmit={handleSubmit} noValidate>
        <NewPasswordFields
          password={password}
          confirmation={confirmation}
          estimate={estimate}
          onPasswordChange={setPassword}
          onConfirmationChange={setConfirmation}
          showErrors={submitted}
          disabled={busy}
        />
        {submitted && problemMessage !== null ? <Alert tone="error">{problemMessage}</Alert> : null}
        {error === null ? null : <Alert tone="error">{error}</Alert>}
        <div className="actions">
          <Button type="submit" variant="primary" busy={busy}>
            {busy ? 'Creating vault…' : 'Create vault'}
          </Button>
        </div>
        {busy ? (
          <p className="muted" role="status">
            Deriving encryption keys. This takes a few seconds.
          </p>
        ) : null}
      </form>
    </section>
  );
}
