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
import ErrorPopup from "../components/ErrorPopup";

function CreateVault() {
  const [error, setError] = useState<string | null>(null);

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

  useEffect(() => {
    if (!storageLocation) {
      setStorageError("");
      return;
    }

    let cancelled = false;

    const validateStorage = async () => {
      try {
        await invoke("validate_storage_location", {
          storageLocation,
          vaultName,
        });

        if (!cancelled) {
          setStorageError("");
        }
      } catch (error) {
        if (!cancelled) {
          setStorageError(String(error));
        }
      }
    };

    validateStorage();

    return () => {
      cancelled = true;
    };
  }, [storageLocation, vaultName]);

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
      await invoke("create_vault", {
        vaultName,
        storageLocation,
        password: passwordProtection ? password : null,
      });

      navigate("/vault");
    } catch (error) {
      setError(String(error));
    }
  };

  return (
    <main className="flex min-h-screen w-full flex-row bg-(--color-background)">
      <ErrorPopup
        open={error !== null}
        message={error ?? ""}
        onClose={() => setError(null)}
      />
      <section className="flex flex-1 items-center justify-center p-6">
        <div className="nv-page-enter nv-stagger-1">
          <Brand />
        </div>
      </section>
      <section className="flex flex-1 items-center justify-center p-5">
        <div className="nv-page-enter nv-stagger-2 w-full max-w-100 xl:max-w-112.5 2xl:max-w-125 [animation-delay:100ms]">
          <div className="w-full animate-page-enter">
            <header className="rounded-t-[22px] border border-b-0 border-(--color-border-strong) bg-(--color-surface) px-5 pt-5 text-center xl:px-6 xl:pt-6">
              <h2 className="m-0 text-[28px] font-semibold text-(--color-text-primary) xl:text-[30px] 2xl:text-[32px]">
                Create Vault
              </h2>
              <p className="mb-0 mt-1.5 text-[16px] text-(--color-text-secondary) xl:text-[17px] 2xl:text-[18px]">
                Create your private storage locally.
              </p>
            </header>
            <form
              className="rounded-b-[22px] border border-t-0 border-(--color-border-strong) bg-(--color-surface) px-4 pb-5 pt-3.5 xl:px-5 xl:pb-6 xl:pt-4"
              onSubmit={handleSubmit}
            >
              <div className="mb-4 flex flex-col gap-1.5">
                <label
                  className="text-[15px] text-(--color-text-secondary)"
                  htmlFor="vault-name"
                >
                  Vault name
                </label>
                <input
                  id="vault-name"
                  name="vaultName"
                  className={`h-10 rounded-[9px] border px-3 text-[15px] outline-none transition ${
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
              <div className="mb-4 flex flex-col gap-1.5">
                <label
                  className="text-[15px] text-(--color-text-secondary)"
                  htmlFor="storage-location"
                >
                  Storage location
                </label>
                <div className="flex gap-2">
                  <input
                    id="storage-location"
                    name="storageLocation"
                    className={`h-10 flex-1 rounded-[9px] border px-3 text-[15px] outline-none transition ${
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
                    className="h-10 min-w-18 rounded-[9px] border border-(--color-border) bg-(--color-surface-muted) px-3 text-[15px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                    type="button"
                    onClick={handleBrowse}
                  >
                    Browse
                  </button>
                </div>
              </div>
              <fieldset className="mb-4 border-0 p-0">
                <legend className="mb-1.5 text-[15px] text-(--color-text-secondary)">
                  Protect with a password
                </legend>
                <div className="flex gap-5">
                  <label className="flex cursor-pointer items-center gap-2 text-[15px] text-(--color-text-secondary)">
                    <input
                      className="h-4 w-4 cursor-pointer appearance-none rounded-full border-2 border-(--color-border) transition checked:border-(--color-accent) checked:bg-(--color-accent) checked:shadow-[inset_0_0_0_3px_white]"
                      type="radio"
                      name="passwordProtection"
                      value="yes"
                      checked={passwordProtection}
                      onChange={() => setPasswordProtection(true)}
                    />
                    Yes
                  </label>
                  <label className="flex cursor-pointer items-center gap-2 text-[15px] text-(--color-text-secondary)">
                    <input
                      className="h-4 w-4 cursor-pointer appearance-none rounded-full border-2 border-(--color-border) transition checked:border-(--color-accent) checked:bg-(--color-accent) checked:shadow-[inset_0_0_0_3px_white]"
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
                  <div className="mb-4 flex flex-col gap-1.5">
                    <label
                      className="text-[15px] text-(--color-text-secondary)"
                      htmlFor="password"
                    >
                      Password
                    </label>
                    <div className="flex gap-2">
                      <input
                        id="password"
                        name="password"
                        className={`h-10 flex-1 rounded-[9px] border px-3 text-[15px] outline-none transition ${
                          firstError && firstError === errors.password
                            ? "border-(--color-danger) shadow-[0_0_0_2px_rgba(225,29,72,0.15)]"
                            : "border-(--color-border) bg-(--color-surface-muted) text-(--color-text-primary) focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                        }`}
                        type={showPassword ? "text" : "password"}
                        value={password}
                        onChange={(event) => setPassword(event.target.value)}
                      />
                      <button
                        className="flex h-10 w-10.5 items-center justify-center rounded-[9px] bg-transparent text-(--color-text-secondary)"
                        type="button"
                        onClick={() => setShowPassword(!showPassword)}
                      >
                        {showPassword ? (
                          <EyeOff className="h-5 w-5" />
                        ) : (
                          <Eye className="h-5 w-5" />
                        )}
                      </button>
                    </div>
                  </div>
                  <div className="mb-4 flex flex-col gap-1.5">
                    <label
                      className="text-[15px] text-(--color-text-secondary)"
                      htmlFor="confirm-password"
                    >
                      Confirm password
                    </label>
                    <div className="flex gap-2">
                      <input
                        id="confirm-password"
                        name="confirmPassword"
                        className={`h-10 flex-1 rounded-[9px] border px-3 text-[15px] outline-none transition ${
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
                        className="flex h-10 w-10.5 items-center justify-center rounded-[9px] bg-transparent text-(--color-text-secondary)"
                        type="button"
                        onClick={() =>
                          setShowConfirmPassword(!showConfirmPassword)
                        }
                      >
                        {showConfirmPassword ? (
                          <EyeOff className="h-5 w-5" />
                        ) : (
                          <Eye className="h-5 w-5" />
                        )}
                      </button>
                    </div>
                  </div>
                </>
              )}
              {firstError && (
                <p className="mb-3 text-[13px] leading-[1.4] text-(--color-danger)">
                  {firstError}
                </p>
              )}
              <div className="nv-fade-up nv-stagger-3 mt-5 flex justify-end gap-2.5 [animation-delay:300ms]">
                <button
                  className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-3 py-1 text-[18px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                  type="button"
                  onClick={() => navigate("/get-started")}
                >
                  Back
                </button>
                <button
                  className={`rounded-2xl px-3 py-1 text-[18px] text-(--color-text-on-accent) transition ${
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
        </div>
      </section>
    </main>
  );
}

export default CreateVault;
