import type { ReactNode } from 'react';

/** Kind of message an {@link Alert} shows. */
export type AlertTone = 'error' | 'success' | 'warning' | 'info';

/** Props of {@link Alert}. */
export interface AlertProps {
  /** Kind of message; decides colour and how assertively it is announced. */
  readonly tone: AlertTone;
  /** The message. */
  readonly children: ReactNode;
}

/**
 * A message box. Errors use `role="alert"` (announced at once); other tones
 * use `role="status"` (announced politely).
 */
export function Alert({ tone, children }: AlertProps) {
  return (
    <div className={`alert alert--${tone}`} role={tone === 'error' ? 'alert' : 'status'}>
      {children}
    </div>
  );
}
