import { useId, type InputHTMLAttributes, type ReactNode, type Ref } from 'react';

/** Props of {@link TextField}. */
export interface TextFieldProps extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  'id' | 'aria-describedby' | 'aria-invalid'
> {
  /** Visible label. */
  readonly label: string;
  /** Help text shown under the input and linked with `aria-describedby`. */
  readonly description?: ReactNode;
  /** Error text; marks the input invalid and is announced with it. */
  readonly error?: string | undefined;
  /** Content placed after the input on the same row (for example a toggle button). */
  readonly trailing?: ReactNode;
  /** Ref to the input element, for focus management. */
  readonly ref?: Ref<HTMLInputElement>;
}

/** A labelled text input with optional help and error text. */
export function TextField({
  label,
  description,
  error,
  trailing,
  className,
  ref,
  ...inputProps
}: TextFieldProps) {
  const id = useId();
  const descriptionId = description === undefined ? undefined : `${id}-description`;
  const errorId = error === undefined ? undefined : `${id}-error`;
  const describedBy = [descriptionId, errorId].filter(Boolean).join(' ') || undefined;

  return (
    <div className={['field', className].filter(Boolean).join(' ')}>
      <label className="field__label" htmlFor={id}>
        {label}
      </label>
      <div className="field__control">
        <input
          {...inputProps}
          ref={ref}
          id={id}
          className="field__input"
          aria-describedby={describedBy}
          aria-invalid={error === undefined ? undefined : true}
        />
        {trailing}
      </div>
      {description === undefined ? null : (
        <div id={descriptionId} className="field__description">
          {description}
        </div>
      )}
      {error === undefined ? null : (
        <p id={errorId} className="field__error">
          {error}
        </p>
      )}
    </div>
  );
}
