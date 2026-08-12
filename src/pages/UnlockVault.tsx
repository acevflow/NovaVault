import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Eye, EyeOff, LockKeyhole } from "lucide-react";
import { useState, useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";

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

  const deriveVaultName = (path: string) => {
    const parts = path.split(/[/\\]/).filter(Boolean);
    return parts.length ? parts[parts.length - 1] : path;
  };

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
    <main className="nv-page-enter flex min-h-screen w-full items-center justify-center bg-(--color-background) py-10 px-4 sm:px-6">
      <section className="w-full max-w-6xl">
        <div className="grid gap-8 lg:grid-cols-[1.05fr_0.95fr]">
          <div className="nv-fade-up nv-stagger-1 rounded-4xl border border-(--color-border-strong) bg-(--color-surface) p-8 shadow-[0_28px_60px_rgba(17,24,39,0.08)]">
            <div className="flex flex-col gap-6 rounded-[30px] border border-(--color-border) bg-(--color-surface-muted) p-6 sm:p-8">
              <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                <div className="flex items-center gap-4">
                  <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-(--color-accent-soft)">
                    <LockKeyhole className="h-7 w-7 text-(--color-accent)" />
                  </div>
                  <div>
                    <p className="text-[13px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                      Unlock Vault
                    </p>
                    <h1 className="mt-2 text-3xl font-semibold text-(--color-text-primary) sm:text-4xl">
                      Secure access in one place
                    </h1>
                  </div>
                </div>
                <div className="rounded-full bg-(--color-background) px-4 py-2 text-[13px] font-medium text-(--color-text-secondary)">
                  {vaultPath ? "Vault selected" : "Select a Vault"}
                </div>
              </div>
              <p className="max-w-2xl text-[15px] leading-relaxed text-(--color-text-secondary)">
                Open a Vault directory and unlock it with a password if needed.
                You can also create a new Vault and return here to access it.
              </p>
            </div>
            <div className="mt-10 space-y-6">
              <div className="rounded-[28px] border border-(--color-border) bg-(--color-surface-muted) p-6">
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <p className="text-[13px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                      Selected Vault
                    </p>
                    <p className="mt-3 text-[20px] font-semibold text-(--color-text-primary)">
                      {vaultPath
                        ? deriveVaultName(vaultPath)
                        : "No vault selected"}
                    </p>
                  </div>
                  <span className="rounded-full bg-(--color-background) px-3 py-1 text-[13px] text-(--color-text-secondary)">
                    {vaultProtected === null
                      ? "Unknown"
                      : vaultProtected
                        ? "Protected"
                        : "Unlocked"}
                  </span>
                </div>
                <p className="mt-4 text-[14px] text-(--color-text-secondary) break-all">
                  {vaultPath ||
                    "Choose a Vault directory from disk or create a new Vault to continue."}
                </p>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <button
                  type="button"
                  onClick={handleBrowseVault}
                  className="rounded-2xl border border-(--color-border) bg-(--color-surface) px-5 py-4 text-[15px] font-medium text-(--color-text-primary) transition hover:bg-(--color-surface-hover)"
                >
                  Open Vault
                </button>
                <button
                  type="button"
                  onClick={() => navigate("/create-vault")}
                  className="rounded-2xl bg-(--color-accent) px-5 py-4 text-[15px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover)"
                >
                  Create Vault
                </button>
              </div>
              {vaultSelectionError && (
                <div className="rounded-2xl bg-[#fde8e8] p-4 text-[14px] text-(--color-danger)">
                  {vaultSelectionError}
                </div>
              )}
            </div>
          </div>
          <div className="nv-fade-up nv-stagger-2 rounded-4xl border border-(--color-border-strong) bg-(--color-surface) p-8 shadow-[0_28px_60px_rgba(17,24,39,0.08)]">
            <div className="mb-6">
              <p className="text-[13px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                Secure access
              </p>
              <h2 className="mt-3 text-2xl font-semibold text-(--color-text-primary)">
                Unlock your Vault
              </h2>
              <p className="mt-3 text-[15px] text-(--color-text-secondary)">
                Enter the password if the selected Vault is protected. If not,
                just proceed to unlock.
              </p>
            </div>
            {vaultProtected !== false && (
              <div className="mb-6 rounded-[20px] border border-(--color-border) bg-(--color-surface-muted) p-4 text-[14px] text-(--color-text-secondary)">
                {vaultProtected === null
                  ? "Select a Vault to see whether it needs a password."
                  : vaultProtected
                    ? "This Vault is protected by a password. Enter it below to unlock."
                    : "This Vault does not require a password. You can unlock immediately."}
              </div>
            )}
            <form className="space-y-5" onSubmit={handleUnlock}>
              {vaultProtected && (
                <div className="space-y-3">
                  <label
                    className="block text-[15px] font-medium text-(--color-text-secondary)"
                    htmlFor="vault-password"
                  >
                    Vault Password
                  </label>
                  <div className="flex gap-2 flex-col sm:flex-row">
                    <input
                      id="vault-password"
                      type={showPassword ? "text" : "password"}
                      value={password}
                      onChange={(event) => setPassword(event.target.value)}
                      autoFocus
                      disabled={unlocking}
                      className="h-12 flex-1 rounded-[11px] border border-(--color-border) bg-(--color-surface-muted) px-4 text-[15px] text-(--color-text-primary) outline-none transition focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)] disabled:opacity-60"
                    />
                    <button
                      type="button"
                      disabled={unlocking}
                      aria-label={
                        showPassword ? "Hide password" : "Show password"
                      }
                      className="flex h-12 w-12 items-center justify-center rounded-[11px] border border-(--color-border) bg-(--color-surface) text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-50"
                      onClick={() => setShowPassword(!showPassword)}
                    >
                      {showPassword ? (
                        <EyeOff className="h-5 w-5" />
                      ) : (
                        <Eye className="h-5 w-5" />
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
              <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                <button
                  type="button"
                  disabled={unlocking}
                  className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-4 py-3 text-[16px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-50"
                  onClick={handleBack}
                >
                  Back
                </button>
                <button
                  type="submit"
                  disabled={unlocking || !vaultPath}
                  className="rounded-2xl bg-(--color-accent) px-4 py-3 text-[16px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active) disabled:cursor-not-allowed disabled:opacity-60"
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
