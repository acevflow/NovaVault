import { Alert } from '../components/Alert';
import { Button } from '../components/Button';
import { ScreenHeading } from '../components/ScreenHeading';

/** Shown while the vault status is being read at startup. */
export function LoadingScreen() {
  return (
    <section className="screen screen--narrow" aria-busy="true">
      <p role="status">Opening NovaVault…</p>
    </section>
  );
}

/** Props of {@link StartupErrorScreen}. */
export interface StartupErrorScreenProps {
  /** What went wrong, for the user. */
  readonly message: string;
  /** Tries to read the vault status again. */
  readonly onRetry: () => void;
}

/** Shown when the vault status cannot be read at startup. */
export function StartupErrorScreen({ message, onRetry }: StartupErrorScreenProps) {
  return (
    <section className="screen screen--narrow">
      <ScreenHeading>NovaVault could not start</ScreenHeading>
      <Alert tone="error">{message}</Alert>
      <div className="actions">
        <Button variant="primary" onClick={onRetry}>
          Try again
        </Button>
      </div>
    </section>
  );
}
