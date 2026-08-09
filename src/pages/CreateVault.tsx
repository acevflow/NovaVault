import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Eye, EyeOff } from "lucide-react";
import { useNavigate } from "react-router-dom";
import {
  validateVaultForm,
  type VaultFormErrors,
} from "../utils/vaultValidation";
import Brand from "../components/Brand";

function CreateVault() {
  const navigate = useNavigate();

  const [vaultName, setVaultName] = useState("");
  const [storageLocation, setStorageLocation] = useState("");
  const [passwordProtection, setPasswordProtection] = useState(true);
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  const [storageError, setStorageError] = useState("");
  const [errors, setErrors] = useState<VaultFormErrors>({});
  const [hasSubmitted, setHasSubmitted] = useState(false);
  const firstError = hasSubmitted
    ? (errors.vaultName ??
      errors.storageLocation ??
      storageError ??
      errors.password ??
      errors.confirmPassword)
    : undefined;

  const handleBrowse = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (!selected) {
      return;
    }

    setStorageLocation(selected);

    try {
      await invoke("validate_storage_location", {
        storageLocation: selected,
      });

      setStorageError("");
    } catch (error) {
      setStorageError(String(error));
    }
  };

  const validationErrors = validateVaultForm({
    vaultName,
    storageLocation,
    passwordProtection,
    password,
    confirmPassword,
  });

  const isFormValid =
    Object.keys(validationErrors).length === 0 && !storageError;

  useEffect(() => {
    if (!hasSubmitted) {
      return;
    }

    const validationErrors = validateVaultForm({
      vaultName,
      storageLocation,
      passwordProtection,
      password,
      confirmPassword,
    });

    setErrors(validationErrors);
  }, [
    vaultName,
    storageLocation,
    passwordProtection,
    password,
    confirmPassword,
    hasSubmitted,
  ]);

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    setHasSubmitted(true);

    const validationErrors = validateVaultForm({
      vaultName,
      storageLocation,
      passwordProtection,
      password,
      confirmPassword,
    });

    setErrors(validationErrors);

    if (Object.keys(validationErrors).length > 0 || storageError) {
      return;
    }
  };

  return (
    <main className="flex min-h-screen w-full flex-col bg-[var(--color-background)] lg:flex-row">
      <section className="flex flex-1 items-center justify-center p-8">
        <Brand />
      </section>
      <section className="flex flex-1 items-center justify-center p-6 sm:p-8">
        <div className="w-full max-w-[450px]">
          <header className="rounded-t-[25px] border border-b-0 border-[color:var(--color-border-strong)] bg-[var(--color-surface)] px-6 pt-6 text-center">
            <h2 className="m-0 text-[32px] font-semibold text-[var(--color-text-primary)]">
              Create Vault
            </h2>
            <p className="mb-0 mt-2 text-[18px] text-[var(--color-text-secondary)]">
              Create your private storage locally.
            </p>
          </header>
          <form
            className="rounded-b-[25px] border border-t-0 border-[color:var(--color-border-strong)] bg-[var(--color-surface)] px-5 pb-6 pt-4"
            onSubmit={handleSubmit}
          >
            <div className="mb-5 flex flex-col gap-2">
              <label className="text-[16px] text-[var(--color-text-secondary)]" htmlFor="vault-name">
                Vault name
              </label>
              <input
                id="vault-name"
                name="vaultName"
                className={`h-[42px] rounded-[10px] border px-3 text-[16px] outline-none transition ${
                  firstError && firstError === errors.vaultName
                    ? "border-[color:var(--color-danger)] shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                    : "border-[color:var(--color-border)] bg-[var(--color-surface-muted)] text-[var(--color-text-primary)] focus:border-[color:var(--color-accent)] focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                }`}
                type="text"
                placeholder="Personal Vault"
                value={vaultName}
                onChange={(event) => setVaultName(event.target.value)}
              />
            </div>
            <div className="mb-5 flex flex-col gap-2">
              <label className="text-[16px] text-[var(--color-text-secondary)]" htmlFor="storage-location">
                Storage location
              </label>
              <div className="flex gap-2">
                <input
                  id="storage-location"
                  name="storageLocation"
                  className={`h-[42px] flex-1 rounded-[10px] border px-3 text-[16px] outline-none transition ${
                    firstError &&
                    (firstError === errors.storageLocation ||
                      firstError === storageError)
                      ? "border-[color:var(--color-danger)] shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                      : "border-[color:var(--color-border)] bg-[var(--color-surface-muted)] text-[var(--color-text-primary)] focus:border-[color:var(--color-accent)] focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                  }`}
                  type="text"
                  value={storageLocation}
                  readOnly
                />
                <button
                  className="h-[42px] min-w-[80px] rounded-[10px] border border-[color:var(--color-border)] bg-[var(--color-surface-muted)] px-3 text-[16px] text-[var(--color-text-secondary)] transition hover:bg-[var(--color-surface-hover)] active:bg-[var(--color-surface-active)]"
                  type="button"
                  onClick={handleBrowse}
                >
                  Browse
                </button>
              </div>
            </div>
            <fieldset className="mb-5 border-0 p-0">
              <legend className="mb-2 text-[16px] text-[var(--color-text-secondary)]">
                Protect with a password
              </legend>
              <div className="flex gap-5">
                <label className="flex cursor-pointer items-center gap-2 text-[var(--color-text-secondary)]">
                  <input
                    className="h-[18px] w-[18px] cursor-pointer appearance-none rounded-full border-2 border-[color:var(--color-border)] transition checked:border-[color:var(--color-accent)] checked:bg-[var(--color-accent)] checked:shadow-[inset_0_0_0_4px_white]"
                    type="radio"
                    name="passwordProtection"
                    value="yes"
                    checked={passwordProtection}
                    onChange={() => setPasswordProtection(true)}
                  />
                  Yes
                </label>
                <label className="flex cursor-pointer items-center gap-2 text-[var(--color-text-secondary)]">
                  <input
                    className="h-[18px] w-[18px] cursor-pointer appearance-none rounded-full border-2 border-[color:var(--color-border)] transition checked:border-[color:var(--color-accent)] checked:bg-[var(--color-accent)] checked:shadow-[inset_0_0_0_4px_white]"
                    type="radio"
                    name="passwordProtection"
                    value="no"
                    checked={!passwordProtection}
                    onChange={() => setPasswordProtection(false)}
                  />
                  No
                </label>
              </div>
            </fieldset>
            {passwordProtection && (
              <>
                <div className="mb-5 flex flex-col gap-2">
                  <label className="text-[16px] text-[var(--color-text-secondary)]" htmlFor="password">
                    Password
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="password"
                      name="password"
                      className={`h-[42px] flex-1 rounded-[10px] border px-3 text-[16px] outline-none transition ${
                        firstError && firstError === errors.password
                          ? "border-[color:var(--color-danger)] shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                          : "border-[color:var(--color-border)] bg-[var(--color-surface-muted)] text-[var(--color-text-primary)] focus:border-[color:var(--color-accent)] focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                      }`}
                      type={showPassword ? "text" : "password"}
                      value={password}
                      onChange={(event) => setPassword(event.target.value)}
                    />
                    <button
                      className="flex h-[42px] w-[45px] items-center justify-center rounded-[10px] bg-transparent text-[var(--color-text-secondary)]"
                      type="button"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <EyeOff className="h-[22px] w-[22px]" />
                      ) : (
                        <Eye className="h-[22px] w-[22px]" />
                      )}
                    </button>
                  </div>
                </div>
                <div className="mb-5 flex flex-col gap-2">
                  <label className="text-[16px] text-[var(--color-text-secondary)]" htmlFor="confirm-password">
                    Confirm password
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="confirm-password"
                      name="confirmPassword"
                      className={`h-[42px] flex-1 rounded-[10px] border px-3 text-[16px] outline-none transition ${
                        firstError && firstError === errors.confirmPassword
                          ? "border-[color:var(--color-danger)] shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                          : "border-[color:var(--color-border)] bg-[var(--color-surface-muted)] text-[var(--color-text-primary)] focus:border-[color:var(--color-accent)] focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                      }`}
                      type={showConfirmPassword ? "text" : "password"}
                      value={confirmPassword}
                      onChange={(event) => setConfirmPassword(event.target.value)}
                    />
                    <button
                      className="flex h-[42px] w-[45px] items-center justify-center rounded-[10px] bg-transparent text-[var(--color-text-secondary)]"
                      type="button"
                      onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                    >
                      {showConfirmPassword ? (
                        <EyeOff className="h-[22px] w-[22px]" />
                      ) : (
                        <Eye className="h-[22px] w-[22px]" />
                      )}
                    </button>
                  </div>
                </div>
              </>
            )}
            {firstError && (
              <p className="mb-4 text-[14px] leading-[1.4] text-[var(--color-danger)]">
                {firstError}
              </p>
            )}
            <div className="mt-6 flex justify-end gap-3">
              <button
                className="rounded-[16px] border border-[color:var(--color-border)] bg-[var(--color-surface-muted)] px-3 py-1 text-[20px] text-[var(--color-text-secondary)] transition hover:bg-[var(--color-surface-hover)] active:bg-[var(--color-surface-active)]"
                type="button"
                onClick={() => navigate("/")}
              >
                Back
              </button>
              <button
                className={`rounded-[16px] px-3 py-1 text-[20px] text-[var(--color-text-on-accent)] transition ${
                  isFormValid
                    ? "bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)]"
                    : "bg-[var(--color-accent-soft)] text-[var(--color-text-inverted)] hover:bg-[var(--color-accent-soft-hover)]"
                }`}
                type="submit"
              >
                Create Vault
              </button>
            </div>
          </form>
        </div>
      </section>
    </main>
  );
}

export default CreateVault;
