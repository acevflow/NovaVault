import type { ButtonHTMLAttributes } from 'react';

/** Visual weight of a {@link Button}. */
export type ButtonVariant = 'primary' | 'secondary' | 'danger';

/** Props of {@link Button}. */
export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual weight; `secondary` by default. */
  readonly variant?: ButtonVariant;
  /**
   * Shows the button as working and disables it. The label should say what
   * is happening (for example "Unlocking…"); `aria-busy` announces it.
   */
  readonly busy?: boolean;
}

/** A button. Defaults to `type="button"` so it never submits a form by accident. */
export function Button({
  variant = 'secondary',
  busy = false,
  disabled,
  className,
  type = 'button',
  ...rest
}: ButtonProps) {
  const classes = ['button', `button--${variant}`, className].filter(Boolean).join(' ');
  return (
    <button
      {...rest}
      type={type}
      className={classes}
      disabled={disabled === true || busy}
      aria-busy={busy || undefined}
    />
  );
}
