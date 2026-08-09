export interface VaultFormData {
  vaultName: string;
  storageLocation: string;
  passwordProtection: boolean;
  password: string;
  confirmPassword: string;
}

export interface VaultFormErrors {
  vaultName?: string;
  storageLocation?: string;
  password?: string;
  confirmPassword?: string;
}

export function validateVaultForm({
  vaultName,
  storageLocation,
  passwordProtection,
  password,
  confirmPassword,
}: VaultFormData): VaultFormErrors {
  const errors: VaultFormErrors = {};

  if (!vaultName.trim()) {
    errors.vaultName = "Vault name is required";
  }

  if (!storageLocation.trim()) {
    errors.storageLocation = "Storage location is required";
  }

  if (passwordProtection) {
    if (!password) {
      errors.password = "Password is required";
    }

    if (!confirmPassword) {
      errors.confirmPassword = "Please confirm your password";
    } else if (password != confirmPassword) {
      errors.confirmPassword = "Passwords do not match";
    }
  }

  return errors;
}
