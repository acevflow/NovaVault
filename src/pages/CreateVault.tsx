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

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
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

    try {
      const vaultPath = await invoke("create_vault", {
        vaultName,
        storageLocation,
      });

      console.log("Vault created:", vaultPath);

      navigate("/vault");
    } catch (error) {
      console.error(error);
    }
  };

  return (
    <main className="flex min-h-screen w-full flex-col bg-(--color-background) lg:flex-row">
      <section className="flex flex-1 items-center justify-center p-8">
        <Brand />
      </section>
      <section className="flex flex-1 items-center justify-center p-6 sm:p-8">
        <div className="w-full max-w-112.5">
          <header className="rounded-t-[25px] border border-b-0 border-(--color-border-strong) bg-(--color-surface) px-6 pt-6 text-center">
            <h2 className="m-0 text-[32px] font-semibold text-(--color-text-primary)">
              Create Vault
            </h2>
            <p className="mb-0 mt-2 text-[18px] text-(--color-text-secondary)">
              Create your private storage locally.
            </p>
          </header>
          <form
            className="rounded-b-[25px] border border-t-0 border-(--color-border-strong) bg-(--color-surface) px-5 pb-6 pt-4"
            onSubmit={handleSubmit}
          >
            <div className="mb-5 flex flex-col gap-2">
              <label
                className="text-[16px] text-(--color-text-secondary)"
                htmlFor="vault-name"
              >
                Vault name
              </label>
              <input
                id="vault-name"
                name="vaultName"
                className={`h-10.5 rounded-[10px] border px-3 text-[16px] outline-none transition ${
                  firstError && firstError === errors.vaultName
                    ? "border-(--color-danger) shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                    : "border-(--color-border) bg-(--color-surface-muted) text-(--color-text-primary) focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                }`}
                type="text"
                placeholder="Personal Vault"
                value={vaultName}
                onChange={(event) => setVaultName(event.target.value)}
              />
            </div>
            <div className="mb-5 flex flex-col gap-2">
              <label
                className="text-[16px] text-(--color-text-secondary)"
                htmlFor="storage-location"
              >
                Storage location
              </label>
              <div className="flex gap-2">
                <input
                  id="storage-location"
                  name="storageLocation"
                  className={`h-10.5 flex-1 rounded-[10px] border px-3 text-[16px] outline-none transition ${
                    firstError &&
                    (firstError === errors.storageLocation ||
                      firstError === storageError)
                      ? "border-(--color-danger) shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                      : "border-(--color-border) bg-(--color-surface-muted) text-(--color-text-primary) focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                  }`}
                  type="text"
                  value={storageLocation}
                  readOnly
                />
                <button
                  className="h-10.5 min-w-20 rounded-[10px] border border-(--color-border) bg-(--color-surface-muted) px-3 text-[16px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                  type="button"
                  onClick={handleBrowse}
                >
                  Browse
                </button>
              </div>
            </div>
            <fieldset className="mb-5 border-0 p-0">
              <legend className="mb-2 text-[16px] text-(--color-text-secondary)">
                Protect with a password
              </legend>
              <div className="flex gap-5">
                <label className="flex cursor-pointer items-center gap-2 text-(--color-text-secondary)">
                  <input
                    className="h-4.5 w-4.5 cursor-pointer appearance-none rounded-full border-2 border-(--color-border) transition checked:border-(--color-accent) checked:bg-(--color-accent) checked:shadow-[inset_0_0_0_4px_white]"
                    type="radio"
                    name="passwordProtection"
                    value="yes"
                    checked={passwordProtection}
                    onChange={() => setPasswordProtection(true)}
                  />
                  Yes
                </label>
                <label className="flex cursor-pointer items-center gap-2 text-(--color-text-secondary)">
                  <input
                    className="h-4.5 w-4.5 cursor-pointer appearance-none rounded-full border-2 border-(--color-border) transition checked:border-(--color-accent) checked:bg-(--color-accent) checked:shadow-[inset_0_0_0_4px_white]"
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
                  <label
                    className="text-[16px] text-(--color-text-secondary)"
                    htmlFor="password"
                  >
                    Password
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="password"
                      name="password"
                      className={`h-10.5 flex-1 rounded-[10px] border px-3 text-[16px] outline-none transition ${
                        firstError && firstError === errors.password
                          ? "border-(--color-danger) shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                          : "border-(--color-border) bg-(--color-surface-muted) text-(--color-text-primary) focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                      }`}
                      type={showPassword ? "text" : "password"}
                      value={password}
                      onChange={(event) => setPassword(event.target.value)}
                    />
                    <button
                      className="flex h-10.5 w-11.25 items-center justify-center rounded-[10px] bg-transparent text-(--color-text-secondary)"
                      type="button"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <EyeOff className="h-5.5 w-5.5" />
                      ) : (
                        <Eye className="h-5.5 w-5.5" />
                      )}
                    </button>
                  </div>
                </div>
                <div className="mb-5 flex flex-col gap-2">
                  <label
                    className="text-[16px] text-(--color-text-secondary)"
                    htmlFor="confirm-password"
                  >
                    Confirm password
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="confirm-password"
                      name="confirmPassword"
                      className={`h-10.5 flex-1 rounded-[10px] border px-3 text-[16px] outline-none transition ${
                        firstError && firstError === errors.confirmPassword
                          ? "border-(--color-danger) shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                          : "border-(--color-border) bg-(--color-surface-muted) text-(--color-text-primary) focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                      }`}
                      type={showConfirmPassword ? "text" : "password"}
                      value={confirmPassword}
                      onChange={(event) =>
                        setConfirmPassword(event.target.value)
                      }
                    />
                    <button
                      className="flex h-10.5 w-11.25 items-center justify-center rounded-[10px] bg-transparent text-(--color-text-secondary)"
                      type="button"
                      onClick={() =>
                        setShowConfirmPassword(!showConfirmPassword)
                      }
                    >
                      {showConfirmPassword ? (
                        <EyeOff className="h-5.5 w-5.5" />
                      ) : (
                        <Eye className="h-5.5 w-5.5" />
                      )}
                    </button>
                  </div>
                </div>
              </>
            )}
            {firstError && (
              <p className="mb-4 text-[14px] leading-[1.4] text-(--color-danger)">
                {firstError}
              </p>
            )}
            <div className="mt-6 flex justify-end gap-3">
              <button
                className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-3 py-1 text-[20px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                type="button"
                onClick={() => navigate("/")}
              >
                Back
              </button>
              <button
                className={`rounded-2xl px-3 py-1 text-[20px] text-(--color-text-on-accent) transition ${
                  isFormValid
                    ? "bg-(--color-accent) hover:bg-(--color-accent-hover)"
                    : "bg-(--color-accent-soft) text-(--color-text-inverted) hover:bg-(--color-accent-soft-hover)"
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
