import { PasswordField } from '../components/PasswordField';
import { StrengthMeter } from './StrengthMeter';
import type { StrengthEstimate } from './strength';

/** Props of {@link NewPasswordFields}. */
export interface NewPasswordFieldsProps {
  /** The new password. */
  readonly password: string;
  /** Its confirmation. */
  readonly confirmation: string;
  /** Strength of {@link NewPasswordFieldsProps.password}, from `usePasswordStrength`. */
  readonly estimate: StrengthEstimate | null;
  /** Called as the password is typed. */
  readonly onPasswordChange: (value: string) => void;
  /** Called as the confirmation is typed. */
  readonly onConfirmationChange: (value: string) => void;
  /** Label of the first field. */
  readonly label?: string;
  /** Whether to show a mismatch error (after the user tried to submit). */
  readonly showErrors: boolean;
  /** Disables both fields. */
  readonly disabled?: boolean;
}

/** A new master password with strength feedback and a confirmation field. */
export function NewPasswordFields({
  password,
  confirmation,
  estimate,
  onPasswordChange,
  onConfirmationChange,
  label = 'Master password',
  showErrors,
  disabled = false,
}: NewPasswordFieldsProps) {
  const mismatch = showErrors && confirmation !== password;
  return (
    <>
      <PasswordField
        label={label}
        value={password}
        autoComplete="new-password"
        required
        disabled={disabled}
        onChange={(event) => {
          onPasswordChange(event.target.value);
        }}
        description={<StrengthMeter estimate={estimate} hasPassword={password !== ''} />}
      />
      <PasswordField
        label={`Confirm ${label.toLowerCase()}`}
        value={confirmation}
        autoComplete="new-password"
        required
        disabled={disabled}
        onChange={(event) => {
          onConfirmationChange(event.target.value);
        }}
        error={mismatch ? 'The passwords do not match.' : undefined}
      />
    </>
  );
}
