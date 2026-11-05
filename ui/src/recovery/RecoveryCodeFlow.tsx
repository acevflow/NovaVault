import { useState, type SubmitEvent } from 'react';

import { Alert } from '../components/Alert';
import { Button } from '../components/Button';
import { ScreenHeading } from '../components/ScreenHeading';
import { TextField } from '../components/TextField';
import { recoveryCodeGroups, recoveryCodesMatch } from './recoveryCode';

/** Props of {@link RecoveryCodeFlow}. */
export interface RecoveryCodeFlowProps {
  /** The code to show, in display form. */
  readonly code: string;
  /** Called once the user has retyped the code correctly. */
  readonly onDone: () => void;
  /** Level of the step headings. */
  readonly headingLevel?: 1 | 2 | 3;
  /** Text shown above the code, explaining when this code was issued. */
  readonly intro?: string;
}

/**
 * Shows a new recovery code once, lets the user print it, and makes them
 * prove they saved it by retyping it.
 *
 * The code exists only in the parent's state and in this component's
 * output; the parent drops it when {@link RecoveryCodeFlowProps.onDone}
 * runs. There is deliberately no copy button: the clipboard is shared with
 * every other app, and the shell's auto-clearing copy only reads item
 * fields from the vault, so a copied code would stay there indefinitely.
 */
export function RecoveryCodeFlow({
  code,
  onDone,
  headingLevel = 1,
  intro = 'This code opens your vault if you forget your master password.',
}: RecoveryCodeFlowProps) {
  const [step, setStep] = useState<'show' | 'confirm'>('show');
  return step === 'show' ? (
    <ShowStep
      code={code}
      intro={intro}
      headingLevel={headingLevel}
      onContinue={() => {
        setStep('confirm');
      }}
    />
  ) : (
    <ConfirmStep
      code={code}
      headingLevel={headingLevel}
      onBack={() => {
        setStep('show');
      }}
      onDone={onDone}
    />
  );
}

interface ShowStepProps {
  readonly code: string;
  readonly intro: string;
  readonly headingLevel: 1 | 2 | 3;
  readonly onContinue: () => void;
}

function ShowStep({ code, intro, headingLevel, onContinue }: ShowStepProps) {
  const [saved, setSaved] = useState(false);
  return (
    <div className="print-sheet stack">
      <ScreenHeading level={headingLevel}>Save your recovery code</ScreenHeading>
      <p className="print-only">
        NovaVault emergency recovery code, printed on {new Date().toLocaleDateString()}.
      </p>
      <p>
        {intro} It is shown only this once. Write it down or print it, and keep it somewhere safe
        and offline. Anyone with this code can open your vault, so print it only on a printer you
        trust.
      </p>
      <ol className="recovery-code" aria-label="Recovery code">
        {recoveryCodeGroups(code).map((group, index) => (
          // Groups can repeat, so the position is part of the key; the list never reorders.
          <li key={`${String(index)}-${group}`}>{group}</li>
        ))}
      </ol>
      <div className="no-print stack">
        <Alert tone="warning">
          If you lose both your master password and this code, your data cannot be recovered.
        </Alert>
        <label className="choice">
          <input
            type="checkbox"
            checked={saved}
            onChange={(event) => {
              setSaved(event.target.checked);
            }}
          />
          I have saved my recovery code somewhere safe
        </label>
        <div className="actions">
          <Button
            onClick={() => {
              window.print();
            }}
          >
            Print
          </Button>
          <Button variant="primary" disabled={!saved} onClick={onContinue}>
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}

interface ConfirmStepProps {
  readonly code: string;
  readonly headingLevel: 1 | 2 | 3;
  readonly onBack: () => void;
  readonly onDone: () => void;
}

function ConfirmStep({ code, headingLevel, onBack, onDone }: ConfirmStepProps) {
  const [typed, setTyped] = useState('');
  const [error, setError] = useState<string | undefined>(undefined);

  function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (recoveryCodesMatch(code, typed)) {
      setTyped('');
      onDone();
    } else {
      setError('That does not match the code shown. Check each group and try again.');
    }
  }

  return (
    <form className="stack" onSubmit={handleSubmit} noValidate>
      <ScreenHeading level={headingLevel}>Confirm your recovery code</ScreenHeading>
      <p>Type the recovery code you saved, to make sure it was written down correctly.</p>
      <TextField
        label="Recovery code"
        className="field--code"
        value={typed}
        autoComplete="off"
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="characters"
        placeholder="XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX"
        description="Dashes, spaces and letter case do not matter."
        error={error}
        onChange={(event) => {
          setTyped(event.target.value);
          setError(undefined);
        }}
      />
      <div className="actions">
        <Button onClick={onBack}>Show the code again</Button>
        <Button type="submit" variant="primary">
          Confirm
        </Button>
      </div>
    </form>
  );
}
