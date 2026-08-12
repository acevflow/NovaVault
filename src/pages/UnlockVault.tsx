import { invoke } from "@tauri-apps/api/core";
import { Eye, EyeOff, LockKeyhole } from "lucide-react";
import { useState, useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import ErrorPopup from "../components/ErrorPopup";

interface UnlockVaultLocationState {
  vaultPath?: string;
}

function UnlockVault() {
  const navigate = useNavigate();
  const location = useLocation();

  const [vaultPath, setVaultPath] = useState<string | null>(null);
  const [loadingVault, setLoadingVault] = useState(true);

  useEffect(() => {
    let cancelled = false;

    const loadVault = async () => {
      try {
        const stateVaultPath = (
          location.state as UnlockVaultLocationState | null
        )?.vaultPath;

        if (stateVaultPath) {
          if (!cancelled) {
            setVaultPath(stateVaultPath);
            setLoadingVault(false);
          }

          return;
        }

        const pendingVault = await invoke<string | null>("get_pending_unlock");

        if (cancelled) {
          return;
        }

        if (!pendingVault) {
          navigate("/", { replace: true });
          return;
        }

        setVaultPath(pendingVault);
      } catch (error) {
        if (!cancelled) {
          setError(String(error));
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
  }, [location.state, navigate]);

  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [unlocking, setUnlocking] = useState(false);

  const handleUnlock = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!vaultPath) {
      setError("No Vault was selected");
      return;
    }

    if (!password) {
      setError("Password is required");
      return;
    }

    setUnlocking(true);

    try {
      const valid = await invoke<boolean>("verify_vault_password", {
        vaultPath,
        password,
      });

      if (!valid) {
        setError("Incorrect password");
        setUnlocking(false);
        return;
      }

      await invoke("open_vault", {
        vaultPath,
        password,
      });

      navigate("/vault", { replace: true });
    } catch (error) {
      setError(String(error));
      setUnlocking(false);
    }
  };

  const handleBack = async () => {
    await invoke("clear_pending_unlock");
    navigate("/", { replace: true });
  };

  if (loadingVault) {
    return (
      <main className="flex min-h-screen w-full items-center justify-center bg-(--color-background)">
        <p className="text-(--color-text-secondary)">Preparing Vault...</p>
      </main>
    );
  }

  return (
    <main className="flex min-h-screen w-full items-center justify-center bg-(--color-background) p-5">
      <section className="w-full max-w-105">
        <div className="rounded-3xl border border-(--color-border-strong) bg-(--color-surface) p-6 shadow-[0_12px_40px_rgba(0,0,0,0.06)] sm:p-8">
          <div className="flex flex-col items-center text-center">
            <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-(--color-accent-soft)">
              <LockKeyhole className="h-7 w-7 text-(--color-accent)" />
            </div>
            <h1 className="mt-4 text-2xl font-semibold text-(--color-text-primary)">
              Unlock Vault
            </h1>
            <p className="mt-2 text-[15px] leading-relaxed text-(--color-text-secondary)">
              Enter your password to open this Vault
            </p>
          </div>
          <form className="mt-6" onSubmit={handleUnlock}>
            <label
              htmlFor="vault-password"
              className="mb-1.5 block text-[15px] text-(--color-text-secondary)"
            >
              Password
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
                className="flex h-10 w-10.5 items-center justify-center rounded-[9px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-50"
                onClick={() => setShowPassword(!showPassword)}
              >
                {showPassword ? (
                  <EyeOff className="h-5 w-5" />
                ) : (
                  <Eye className="h-5 w-5" />
                )}
              </button>
            </div>
            <div className="mt-5 flex justify-end gap-2.5">
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
                disabled={unlocking}
                className="rounded-2xl bg-(--color-accent) px-4 py-2 text-[16px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active) disabled:cursor-not-allowed disabled:opacity-60"
              >
                {unlocking ? "Unlocking..." : "Unlock"}
              </button>
            </div>
          </form>
        </div>
      </section>
      <ErrorPopup
        open={error !== null}
        message={error ?? ""}
        onClose={() => setError(null)}
      />
    </main>
  );
}

export default UnlockVault;
