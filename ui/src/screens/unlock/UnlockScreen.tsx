import { useId, useRef, useState, type KeyboardEvent, type SubmitEvent } from 'react';

import { unlock } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import type { CredentialMethod } from '../../api/types';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { PasswordField } from '../../components/PasswordField';
import { ScreenHeading } from '../../components/ScreenHeading';
import { TextField } from '../../components/TextField';

/** Props of {@link UnlockScreen}. */
export interface UnlockScreenProps {
  /** Called once the vault is unlocked. */
  readonly onUnlocked: () => void;
}

const METHODS: readonly { readonly method: CredentialMethod; readonly label: string }[] = [
  { method: 'password', label: 'Master password' },
  { method: 'recoveryCode', label: 'Recovery code' },
];

/** Unlocks the vault with the master password or, if it is forgotten, the recovery code. */
export function UnlockScreen({ onUnlocked }: UnlockScreenProps) {
  const id = useId();
  const [method, setMethod] = useState<CredentialMethod>('password');
  const [secret, setSecret] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  const input = useRef<HTMLInputElement>(null);

  function selectMethod(next: CredentialMethod) {
    setMethod(next);
    // Never carry a typed secret over to the other method.
    setSecret('');
    setError(null);
  }

  // Arrow keys move between tabs, as in the WAI-ARIA tabs pattern.
  function handleTabKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    const last = METHODS.length - 1;
    const target =
      event.key === 'ArrowRight'
        ? (index + 1) % METHODS.length
        : event.key === 'ArrowLeft'
          ? (index + last) % METHODS.length
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? last
              : null;
    const next = target === null ? undefined : METHODS[target];
    if (target === null || next === undefined) {
      return;
    }
    event.preventDefault();
    selectMethod(next.method);
    tabs.current[target]?.focus();
  }

  async function attempt() {
    setBusy(true);
    setError(null);
    try {
      await unlock({ method, secret });
      setSecret('');
      onUnlocked();
    } catch (reason) {
      // Clear the field after a failure so a wrong secret does not linger.
      setSecret('');
      setError(
        isCommandError(reason, 'invalid_credentials')
          ? method === 'password'
            ? 'That master password is not correct.'
            : 'That recovery code is not correct. It may have been replaced by a newer one.'
          : errorMessage(reason),
      );
      setBusy(false);
      input.current?.focus();
    }
  }

  function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (secret === '') {
      setError(method === 'password' ? 'Enter your master password.' : 'Enter your recovery code.');
      input.current?.focus();
      return;
    }
    void attempt();
  }

  return (
    <section className="screen screen--narrow">
      <ScreenHeading>Unlock your vault</ScreenHeading>
      <div className="tabs" role="tablist" aria-label="Unlock with">
        {METHODS.map((option, index) => (
          <button
            key={option.method}
            ref={(element) => {
              tabs.current[index] = element;
            }}
            type="button"
            role="tab"
            id={`${id}-tab-${option.method}`}
            className="tabs__tab"
            aria-selected={method === option.method}
            aria-controls={`${id}-panel`}
            tabIndex={method === option.method ? 0 : -1}
            onClick={() => {
              selectMethod(option.method);
            }}
            onKeyDown={(event) => {
              handleTabKeyDown(event, index);
            }}
          >
            {option.label}
          </button>
        ))}
      </div>
      <div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-tab-${method}`}>
        <form className="stack" onSubmit={handleSubmit} noValidate>
          {method === 'password' ? (
            <PasswordField
              key="password"
              ref={input}
              label="Master password"
              value={secret}
              autoComplete="current-password"
              disabled={busy}
              onChange={(event) => {
                setSecret(event.target.value);
              }}
            />
          ) : (
            <TextField
              key="recoveryCode"
              ref={input}
              label="Recovery code"
              className="field--code"
              value={secret}
              autoComplete="off"
              spellCheck={false}
              autoCorrect="off"
              autoCapitalize="characters"
              disabled={busy}
              description="Forgot your master password? Unlock with your recovery code, then set a new password in Settings."
              onChange={(event) => {
                setSecret(event.target.value);
              }}
            />
          )}
          {error === null ? null : <Alert tone="error">{error}</Alert>}
          <div className="actions">
            <Button type="submit" variant="primary" busy={busy}>
              {busy ? 'Unlocking…' : 'Unlock'}
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}
