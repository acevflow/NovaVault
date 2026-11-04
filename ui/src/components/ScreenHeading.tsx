import { useEffect, useRef, type ReactNode } from 'react';

/** Props of {@link ScreenHeading}. */
export interface ScreenHeadingProps {
  /** Heading text. */
  readonly children: ReactNode;
  /** Heading level: 1 for a screen, 2 or 3 for a step inside a section. */
  readonly level?: 1 | 2 | 3;
}

/**
 * The heading of a screen or of a step in a flow. It takes focus when it
 * appears, so screen reader and keyboard users land at the top of the new
 * content instead of on a control that no longer exists.
 */
export function ScreenHeading({ children, level = 1 }: ScreenHeadingProps) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);
  const Tag = `h${String(level)}` as 'h1' | 'h2' | 'h3';
  return (
    <Tag ref={ref} tabIndex={-1} className={level === 1 ? 'screen__heading' : 'step__heading'}>
      {children}
    </Tag>
  );
}
