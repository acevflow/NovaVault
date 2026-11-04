import { useId, type ReactNode, type TextareaHTMLAttributes } from 'react';

/** Props of {@link TextAreaField}. */
export interface TextAreaFieldProps extends Omit<
  TextareaHTMLAttributes<HTMLTextAreaElement>,
  'id' | 'aria-describedby'
> {
  /** Visible label. */
  readonly label: string;
  /** Help text shown under the input and linked with `aria-describedby`. */
  readonly description?: ReactNode;
  /** Content placed under the input (for example a button that fills it). */
  readonly trailing?: ReactNode;
}

/**
 * A labelled multi-line text input. Spell checking and autocorrection are
 * off: notes may hold secrets, which must not reach a dictionary service.
 */
export function TextAreaField({
  label,
  description,
  trailing,
  className,
  ...textareaProps
}: TextAreaFieldProps) {
  const id = useId();
  const descriptionId = description === undefined ? undefined : `${id}-description`;
  return (
    <div className={['field', className].filter(Boolean).join(' ')}>
      <label className="field__label" htmlFor={id}>
        {label}
      </label>
      <textarea
        rows={4}
        spellCheck={false}
        autoCorrect="off"
        autoCapitalize="off"
        {...textareaProps}
        id={id}
        className="field__input field__textarea"
        aria-describedby={descriptionId}
      />
      {description === undefined ? null : (
        <div id={descriptionId} className="field__description">
          {description}
        </div>
      )}
      {trailing}
    </div>
  );
}
