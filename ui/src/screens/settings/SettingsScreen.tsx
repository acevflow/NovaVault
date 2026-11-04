import { ScreenHeading } from '../../components/ScreenHeading';
import { AppearanceSection } from './AppearanceSection';
import { AutoLockSection } from './AutoLockSection';
import { ChangePasswordSection } from './ChangePasswordSection';
import { RecoveryCodeSection } from './RecoveryCodeSection';

/** Props of {@link SettingsScreen}. */
export interface SettingsScreenProps {
  /** Called if a command reports that the vault is locked. */
  readonly onLocked: () => void;
}

/** Security and appearance settings of the unlocked vault. */
export function SettingsScreen({ onLocked }: SettingsScreenProps) {
  return (
    <div className="screen">
      <ScreenHeading>Settings</ScreenHeading>
      <ChangePasswordSection onLocked={onLocked} />
      <RecoveryCodeSection onLocked={onLocked} />
      <AutoLockSection onLocked={onLocked} />
      <AppearanceSection />
    </div>
  );
}
