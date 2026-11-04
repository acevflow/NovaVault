import { useId, type Ref } from 'react';

import type { CredentialMethod } from '../../api/types';
import { PasswordField } from '../../components/PasswordField';
import { TextField } from '../../components/TextField';

/** Props of {@link CurrentCredentialFields}. */
export interface CurrentCredentialFieldsProps {
  /** Which secret the user confirms with. */
  readonly method: CredentialMethod;
  /** The typed secret. */
  readonly secret: string;
  /** Called when the user picks the other method; the parent should clear the secret. */
  readonly onMethodChange: (method: CredentialMethod) => void;
  /** Called as the secret is typed. */
  readonly onSecretChange: (secret: string) => void;
  /** Disables the fields. */
  readonly disabled?: boolean;
  /** Ref to the secret input, to focus it after an error. */
  readonly inputRef?: Ref<HTMLInputElement>;
}

/**
 * Re-authentication before a key slot change: the current master password,
 * or the recovery code for users who unlocked with it because they forgot
 * the password.
 */
export function CurrentCredentialFields({
  method,
  secret,
  onMethodChange,
  onSecretChange,
  disabled = false,
  inputRef,
}: CurrentCredentialFieldsProps) {
  const name = useId();
  const options: readonly { readonly value: CredentialMethod; readonly label: string }[] = [
    { value: 'password', label: 'Master password' },
    { value: 'recoveryCode', label: 'Recovery code' },
  ];
  return (
    <>
      <fieldset className="fieldset">
        <legend>Confirm it is you with</legend>
        <div className="choice-row">
          {options.map((option) => (
            <label key={option.value} className="choice">
              <input
                type="radio"
                name={name}
                value={option.value}
                checked={method === option.value}
                disabled={disabled}
                onChange={() => {
                  onMethodChange(option.value);
                }}
              />
              {option.label}
            </label>
          ))}
        </div>
      </fieldset>
      {method === 'password' ? (
        <PasswordField
          key="password"
          ref={inputRef}
          label="Current master password"
          value={secret}
          autoComplete="current-password"
          disabled={disabled}
          onChange={(event) => {
            onSecretChange(event.target.value);
          }}
        />
      ) : (
        <TextField
          key="recoveryCode"
          ref={inputRef}
          label="Current recovery code"
          className="field--code"
          value={secret}
          autoComplete="off"
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="characters"
          disabled={disabled}
          onChange={(event) => {
            onSecretChange(event.target.value);
          }}
        />
      )}
    </>
  );
}
