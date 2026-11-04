import { useEffect, useRef, type ReactNode } from 'react';

/** Props of {@link ScreenHeading}. */
export interface ScreenHeadingProps {
  /** Heading text. */
  readonly children: ReactNode;
}

/**
 * The `<h1>` of a screen. It takes focus when the screen appears, so screen
 * reader and keyboard users land at the top of the new content instead of
 * on a control that no longer exists.
 */
export function ScreenHeading({ children }: ScreenHeadingProps) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);
  return (
    <h1 ref={ref} tabIndex={-1} className="screen__heading">
      {children}
    </h1>
  );
}
