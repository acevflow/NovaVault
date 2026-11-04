import { useState } from 'react';

import { TextField, type TextFieldProps } from './TextField';

/** Props of {@link PasswordField}. */
export type PasswordFieldProps = Omit<TextFieldProps, 'type' | 'trailing'>;

/**
 * A password input with a show/hide toggle.
 *
 * Spell checking and autocorrection are off so the browser never sends the
 * value to a dictionary service or learns it.
 */
export function PasswordField({ label, ...rest }: PasswordFieldProps) {
  const [visible, setVisible] = useState(false);
  return (
    <TextField
      {...rest}
      label={label}
      type={visible ? 'text' : 'password'}
      spellCheck={false}
      autoCorrect="off"
      autoCapitalize="off"
      trailing={
        <button
          type="button"
          className="field__toggle"
          aria-pressed={visible}
          aria-label={`Show ${label.toLowerCase()}`}
          onClick={() => {
            setVisible((current) => !current);
          }}
        >
          {visible ? 'Hide' : 'Show'}
        </button>
      }
    />
  );
}
