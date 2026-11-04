import { useEffect, useId, useState, type SubmitEvent } from 'react';

import { getSettings, setClipboardClearDelay } from '../../api/commands';
import { errorMessage, isCommandError } from '../../api/errors';
import { Alert } from '../../components/Alert';
import { Button } from '../../components/Button';
import { formatSeconds } from '../../format/duration';

/** Choices offered for the clipboard clear delay, in seconds (the shell accepts 5 to 300). */
const CLEAR_CHOICES: readonly number[] = [10, 30, 60, 120];

/** Props of {@link ClipboardSection}. */
export interface ClipboardSectionProps {
  /** Called if the vault turns out to be locked. */
  readonly onLocked: () => void;
}

/** Sets how long a copied field stays on the clipboard. */
export function ClipboardSection({ onLocked }: ClipboardSectionProps) {
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
        setSaved(settings.clipboardClearSeconds);
        setSelected(settings.clipboardClearSeconds);
      })
      .catch((reason: unknown) => {
        if (active) setError(errorMessage(reason));
      });
    return () => {
      active = false;
    };
  }, []);

  async function save(seconds: number) {
    setBusy(true);
    try {
      await setClipboardClearDelay(seconds);
      setSaved(seconds);
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
    saved === null || CLEAR_CHOICES.includes(saved)
      ? CLEAR_CHOICES
      : [...CLEAR_CHOICES, saved].sort((a, b) => a - b);

  return (
    <section className="card" aria-labelledby={headingId}>
      <h2 id={headingId}>Clipboard</h2>
      <p className="muted">
        Copied passwords and other fields are removed from the clipboard after this delay, and when
        the vault locks, unless you have copied something else since.
      </p>
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
              Clear the clipboard after
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
              {choices.map((seconds) => (
                <option key={seconds} value={seconds}>
                  {formatSeconds(seconds)}
                </option>
              ))}
            </select>
          </div>
          {error === null ? null : <Alert tone="error">{error}</Alert>}
          {done && saved !== null ? (
            <Alert tone="success">
              Copied fields are now cleared after {formatSeconds(saved)}.
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
