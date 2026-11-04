import { ScreenHeading } from '../components/ScreenHeading';

/** The unlocked vault's start screen. Items are listed here in a later release. */
export function HomeScreen() {
  return (
    <section className="screen">
      <ScreenHeading>Your vault</ScreenHeading>
      <p>The vault is unlocked. Passwords, notes and secrets will be listed here.</p>
      <p className="muted">Lock the vault when you step away. It also locks itself when idle.</p>
    </section>
  );
}
