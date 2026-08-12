import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Eye, EyeOff, LockKeyhole } from "lucide-react";
import { useState, useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import Brand from "../components/Brand";
import ErrorPopup from "../components/ErrorPopup";

interface UnlockVaultLocationState {
  vaultPath?: string;
}

function UnlockVault() {
  const navigate = useNavigate();
  const location = useLocation();

  const [vaultPath, setVaultPath] = useState<string | null>(null);
  const [vaultProtected, setVaultProtected] = useState<boolean | null>(null);
  const [vaultSelectionError, setVaultSelectionError] = useState<string | null>(
    null,
  );
  const [password, setPassword] = useState("");
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const [loadingVault, setLoadingVault] = useState(true);
  const [unlocking, setUnlocking] = useState(false);

  const updateVaultSelection = async (path: string | null) => {
    setVaultPath(path);
    setVaultSelectionError(null);
    setPasswordError(null);

    if (!path) {
      setVaultProtected(null);
      return;
    }

    try {
      const isProtected = await invoke<boolean>("is_vault_password_protected", {
        vaultPath: path,
      });

      setVaultProtected(isProtected);
    } catch (error) {
      setVaultProtected(null);
      setVaultSelectionError(String(error));
    }
  };

  useEffect(() => {
    let cancelled = false;

    const loadVault = async () => {
      try {
        const stateVaultPath = (
          location.state as UnlockVaultLocationState | null
        )?.vaultPath;

        if (stateVaultPath) {
          if (!cancelled) {
            await updateVaultSelection(stateVaultPath);
            setLoadingVault(false);
          }

          return;
        }

        const pendingVault = await invoke<string | null>("get_pending_unlock");

        if (cancelled) {
          return;
        }

        if (!pendingVault) {
          if (!cancelled) {
            await updateVaultSelection(null);
            setLoadingVault(false);
          }
          return;
        }

        if (!cancelled) {
          await updateVaultSelection(pendingVault);
        }
      } catch (error) {
        if (!cancelled) {
          setVaultSelectionError(String(error));
        }
      } finally {
        if (!cancelled) {
          setLoadingVault(false);
        }
      }
    };

    loadVault();

    return () => {
      cancelled = true;
    };
  }, [location.state]);

  const handleBrowseVault = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (!selected) {
      return;
    }

    const path = Array.isArray(selected) ? selected[0] : selected;

    if (typeof path !== "string") {
      return;
    }

    await updateVaultSelection(path);
  };

  const handleUnlock = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setVaultSelectionError(null);
    setPasswordError(null);

    if (!vaultPath) {
      setVaultSelectionError("Select a Vault to unlock.");
      return;
    }

    if (vaultProtected) {
      if (!password) {
        setPasswordError("Password is required.");
        return;
      }

      setUnlocking(true);

      try {
        const valid = await invoke<boolean>("verify_vault_password", {
          vaultPath,
          password,
        });

        if (!valid) {
          setPasswordError("Incorrect password.");
          setUnlocking(false);
          return;
        }

        await invoke("open_vault", {
          vaultPath,
          password,
        });

        navigate("/vault", { replace: true });
      } catch (error) {
        const message = String(error);

        if (message.includes("This Vault is protected by a password")) {
          setPasswordError("Incorrect password.");
        } else {
          setVaultSelectionError(message);
        }

        setUnlocking(false);
      }
    } else {
      setUnlocking(true);

      try {
        await invoke("open_vault", {
          vaultPath,
        });

        navigate("/vault", { replace: true });
      } catch (error) {
        setVaultSelectionError(String(error));
        setUnlocking(false);
      }
    }
  };

  const handleBack = async () => {
    await invoke("clear_pending_unlock");
    navigate("/", { replace: true });
  };

  if (loadingVault) {
    return (
      <main className="nv-page-enter flex min-h-screen w-full items-center justify-center bg-(--color-background) p-5">
        <p className="text-(--color-text-secondary)">Preparing Vault...</p>
      </main>
    );
  }

  return (
    <main className="flex min-h-screen w-full flex-row bg-(--color-background)">
      <ErrorPopup
        open={vaultSelectionError !== null}
        message={vaultSelectionError ?? ""}
        onClose={() => setVaultSelectionError(null)}
      />
      <section className="flex flex-1 items-center justify-center p-6">
        <div className="nv-page-enter nv-stagger-1">
          <Brand />
        </div>
      </section>

      <section className="flex flex-1 items-center justify-center p-5">
        <div className="nv-page-enter nv-stagger-2 w-full max-w-[32rem]">
          <div className="w-full animate-page-enter">
            <header className="rounded-t-[22px] border border-b-0 border-(--color-border-strong) bg-(--color-surface) px-5 pt-5 text-center xl:px-6 xl:pt-6">
              <div className="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-(--color-accent-soft)">
                <LockKeyhole className="h-5 w-5 text-(--color-accent)" />
              </div>
              <h2 className="m-0 text-[28px] font-semibold text-(--color-text-primary) xl:text-[30px] 2xl:text-[32px]">
                Unlock Vault
              </h2>
              <p className="mb-0 mt-1.5 text-[16px] text-(--color-text-secondary) xl:text-[17px] 2xl:text-[18px]">
                Pick your Vault and verify access.
              </p>
            </header>

            <form
              className="rounded-b-[22px] border border-t-0 border-(--color-border-strong) bg-(--color-surface) px-4 pb-5 pt-3.5 xl:px-5 xl:pb-6 xl:pt-4"
              onSubmit={handleUnlock}
            >
              <div className="mb-4 flex flex-col gap-1.5">
                <label
                  className="text-[15px] text-(--color-text-secondary)"
                  htmlFor="vault-path"
                >
                  Vault location
                </label>
                <div className="flex gap-2">
                  <input
                    id="vault-path"
                    type="text"
                    value={vaultPath ?? ""}
                    readOnly
                    placeholder="Select a Vault folder"
                    className="h-10 flex-1 rounded-[9px] border border-(--color-border) bg-(--color-surface-muted) px-3 text-[15px] text-(--color-text-primary) outline-none transition focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)]"
                  />
                  <button
                    type="button"
                    className="h-10 min-w-18 rounded-[9px] border border-(--color-border) bg-(--color-surface-muted) px-3 text-[15px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                    onClick={handleBrowseVault}
                  >
                    Browse
                  </button>
                </div>
              </div>

              {vaultProtected !== false && (
                <div className="mb-4 rounded-[12px] border border-(--color-border) bg-(--color-surface-muted) px-3 py-2 text-[14px] leading-relaxed text-(--color-text-secondary)">
                  {vaultProtected === null
                    ? "Select a Vault to see whether it needs a password."
                    : vaultProtected
                      ? "This Vault is protected by a password. Enter it below."
                      : "This Vault is already unlocked and can be opened immediately."}
                </div>
              )}

              {vaultProtected && (
                <div className="mb-4 flex flex-col gap-1.5">
                  <label
                    className="text-[15px] text-(--color-text-secondary)"
                    htmlFor="vault-password"
                  >
                    Vault password
                  </label>
                  <div className="flex gap-2">
                    <input
                      id="vault-password"
                      type={showPassword ? "text" : "password"}
                      value={password}
                      onChange={(event) => setPassword(event.target.value)}
                      autoFocus
                      disabled={unlocking}
                      className="h-10 flex-1 rounded-[9px] border border-(--color-border) bg-(--color-surface-muted) px-3 text-[15px] text-(--color-text-primary) outline-none transition focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)] disabled:opacity-60"
                    />
                    <button
                      type="button"
                      disabled={unlocking}
                      aria-label={showPassword ? "Hide password" : "Show password"}
                      className="flex h-10 w-10 items-center justify-center rounded-[9px] border border-(--color-border) bg-(--color-surface) text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-50"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <EyeOff className="h-4 w-4" />
                      ) : (
                        <Eye className="h-4 w-4" />
                      )}
                    </button>
                  </div>
                  {passwordError && (
                    <p className="text-[14px] text-(--color-danger)">
                      {passwordError}
                    </p>
                  )}
                </div>
              )}

              {vaultSelectionError && (
                <div className="mb-4 rounded-[10px] bg-[#fee2e2] p-3 text-[14px] text-(--color-danger)">
                  {vaultSelectionError}
                </div>
              )}

              <div className="mt-5 flex items-center justify-between gap-3">
                <button
                  type="button"
                  disabled={unlocking}
                  className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-4 py-2 text-[16px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-50"
                  onClick={handleBack}
                >
                  Back
                </button>
                <button
                  type="submit"
                  disabled={unlocking || !vaultPath}
                  className="rounded-2xl bg-(--color-accent) px-4 py-2 text-[16px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active) disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {unlocking ? "Unlocking..." : "Unlock Vault"}
                </button>
              </div>
            </form>
          </div>
        </div>
      </section>
    </main>
  );
}

export default UnlockVault;
