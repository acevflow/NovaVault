import { useId } from 'react';

import { MIN_ACCEPTED_SCORE, STRENGTH_LABELS, type StrengthEstimate } from './strength';

/** Props of {@link StrengthMeter}. */
export interface StrengthMeterProps {
  /** The estimate to show, or `null` while none is available. */
  readonly estimate: StrengthEstimate | null;
  /** Whether the user has typed anything yet. */
  readonly hasPassword: boolean;
}

/**
 * Visual and announced password strength, with tips to improve it.
 *
 * The label is a polite live region, so screen reader users hear the
 * strength change as they type without being interrupted.
 */
export function StrengthMeter({ estimate, hasPassword }: StrengthMeterProps) {
  const labelId = useId();
  if (!hasPassword) {
    return (
      <p className="field__description">
        Use a long passphrase of several unrelated words. It must rate at least “
        {STRENGTH_LABELS[MIN_ACCEPTED_SCORE]}”.
      </p>
    );
  }
  const score = estimate?.score ?? 0;
  const tips = (estimate === null ? [] : [estimate.warning, ...estimate.suggestions]).filter(
    (tip): tip is string => tip !== null && tip !== '',
  );
  return (
    <div className="strength">
      <meter
        className="strength__meter"
        min={0}
        max={4}
        low={2}
        high={3}
        optimum={4}
        value={score}
        aria-labelledby={labelId}
      />
      <p id={labelId} className="strength__label" aria-live="polite">
        Password strength: {estimate === null ? 'checking…' : STRENGTH_LABELS[score]}
      </p>
      {tips.length === 0 ? null : (
        <ul className="strength__tips">
          {tips.map((tip) => (
            <li key={tip}>{tip}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
