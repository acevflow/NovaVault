import { useEffect, useId, useState, type SubmitEvent } from 'react';

import { getSettings, setAutoLockTimeout } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';

/** Choices offered for the inactivity timeout, in minutes (the shell accepts 1 to 60). */
const AUTO_LOCK_CHOICES: readonly number[] = [1, 5, 15, 30, 60];

/** Props of {@link AutoLockSection}. */
export interface AutoLockSectionProps {
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

function minutesLabel(minutes: number): string {
  if (minutes === 60) return '1 hour';
  return minutes === 1 ? '1 minute' : `${String(minutes)} minutes`;
}

/** Sets how long the vault may stay unlocked without activity. */
export function AutoLockSection({ onLocked }: AutoLockSectionProps) {
  const headingId = useId();
  const selectId = useId();
  const [saved, setSaved] = useState<number | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  useEffect(() => {
    let active = true;
    getSettings()
      .then((settings) => {
        if (!active) return;
        setSaved(settings.autoLockMinutes);
        setSelected(settings.autoLockMinutes);
      })
      .catch((reason: unknown) => {
        if (active) setError(errorMessage(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  async function save(minutes: number) {
    setBusy(true);
    try {
      await setAutoLockTimeout(minutes);
      setSaved(minutes);
      setDone(true);
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

  function handleSubmit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setDone(false);
    if (selected !== null) {
      void save(selected);
    }
  }

  // Keep a value set outside the usual choices selectable.
  const choices =
    saved === null || AUTO_LOCK_CHOICES.includes(saved)
      ? AUTO_LOCK_CHOICES
      : [...AUTO_LOCK_CHOICES, saved].sort((a, b) => a - b);

  return (
    <section className="card" aria-labelledby={headingId}>
      <h2 id={headingId}>Auto-lock</h2>
      <p className="muted">The vault also locks when the computer sleeps or the screen locks.</p>
      {selected === null ? (
        error === null ? (
          <p role="status">Loading settings…</p>
        ) : (
          <Alert tone="error">{error}</Alert>
        )
      ) : (
        <form className="stack" onSubmit={handleSubmit}>
          <div className="field">
            <label className="field__label" htmlFor={selectId}>
              Lock after inactivity
            </label>
            <select
              id={selectId}
              className="select"
              value={selected}
              disabled={busy}
              onChange={(event) => {
                setSelected(Number(event.target.value));
                setDone(false);
              }}
            >
              {choices.map((minutes) => (
                <option key={minutes} value={minutes}>
                  {minutesLabel(minutes)}
                </option>
              ))}
            </select>
          </div>
          {error === null ? null : <Alert tone="error">{error}</Alert>}
          {done && saved !== null ? (
            <Alert tone="success">
              The vault now locks after {minutesLabel(saved)} without activity.
            </Alert>
          ) : null}
          <div className="actions">
            <Button type="submit" variant="primary" busy={busy} disabled={selected === saved}>
              {busy ? 'Saving…' : 'Save'}
            </Button>
          </div>
        </form>
      )}
    </section>
  );
}
