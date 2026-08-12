import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Plus, FolderOpen, Lock } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import ErrorPopup from "../components/ErrorPopup";

interface SavedVault {
  id: string;
  name: string;
  path: string;
}

function Vault() {
  const [error, setError] = useState<string | null>(null);

  const navigate = useNavigate();

  const [vaults, setVaults] = useState<SavedVault[]>([]);
  const [openVault, setOpenVault] = useState<SavedVault | null>(null);
  const [passwordPromptVault, setPasswordPromptVault] =
    useState<SavedVault | null>(null);
  const [passwordPrompt, setPasswordPrompt] = useState("");
  const [passwordPromptError, setPasswordPromptError] = useState<string | null>(
    null,
  );
  const [passwordPromptLoading, setPasswordPromptLoading] = useState(false);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const loadVaults = async () => {
      try {
        const savedVaults = await invoke<SavedVault[]>("get_saved_vaults");

        setVaults(savedVaults);

        const currentPath = await invoke<string | null>("get_open_vault");

        if (currentPath) {
          const currentVault = savedVaults.find(
            (vault) => vault.path === currentPath,
          );

          if (currentVault) {
            setOpenVault(currentVault);
          }
        }
      } catch (error) {
        setError(String(error));
      }
    };

    loadVaults();
  }, []);

  const handleSwitchVault = async (vault: SavedVault) => {
    if (loading) {
      return;
    }

    setLoading(true);

    try {
      await invoke("open_vault", {
        vaultPath: vault.path,
      });

      setOpenVault(vault);
    } catch (error) {
      const message = String(error);

      if (message.includes("This Vault is protected by a password")) {
        setPasswordPromptError(null);
        setPasswordPrompt("");
        setPasswordPromptVault(vault);
        return;
      }

      if (message.includes("does not exist")) {
        setVaults((previous) =>
          previous.filter((item) => item.id !== vault.id),
        );
        setError(
          "The selected Vault no longer exists and was removed from the list",
        );
        return;
      }

      setError(message);
    } finally {
      setLoading(false);
    }
  };

  const handlePasswordPromptSubmit = async (
    event: React.FormEvent<HTMLFormElement>,
  ) => {
    event.preventDefault();

    if (!passwordPromptVault) {
      return;
    }

    setPasswordPromptLoading(true);
    setPasswordPromptError(null);

    try {
      await invoke("open_vault", {
        vaultPath: passwordPromptVault.path,
        password: passwordPrompt,
      });

      setOpenVault(passwordPromptVault);
      setPasswordPromptVault(null);
      setPasswordPrompt("");
    } catch (error) {
      const message = String(error);

      if (message.includes("Incorrect Vault password")) {
        setPasswordPromptError("Incorrect password");
        return;
      }

      if (message.includes("does not exist")) {
        setVaults((previous) =>
          previous.filter((item) => item.id !== passwordPromptVault.id),
        );
        setPasswordPromptVault(null);
        setError(
          "The selected Vault no longer exists and was removed from the list",
        );
        return;
      }

      setPasswordPromptError(message);
    } finally {
      setPasswordPromptLoading(false);
    }
  };

  const handlePasswordPromptCancel = () => {
    setPasswordPromptVault(null);
    setPasswordPrompt("");
    setPasswordPromptError(null);
  };

  const handleOpenVault = async () => {
    if (loading) {
      return;
    }

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

    setLoading(true);

    try {
      await invoke("open_vault", {
        vaultPath: path,
      });

      const updatedVaults = await invoke<SavedVault[]>("get_saved_vaults");

      setVaults(updatedVaults);

      const currentVault = updatedVaults.find((vault) => vault.path === path);

      if (currentVault) {
        setOpenVault(currentVault);
      }
    } catch (error) {
      const message = String(error);

      if (message.includes("This Vault is protected by a password")) {
        setPasswordPromptError(null);
        setPasswordPrompt("");
        setPasswordPromptVault({
          id: path,
          name: path.split(/[/\\]/).pop() ?? path,
          path,
        });
      } else {
        console.error("Failed to open Vault:", error);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleLock = async () => {
    if (loading) {
      return;
    }

    setLoading(true);

    try {
      const nextVaultPath = await invoke<string | null>("close_vault");

      if (nextVaultPath) {
        const nextVault = vaults.find((vault) => vault.path === nextVaultPath);

        if (nextVault) {
          setOpenVault(nextVault);
        }
      } else {
        navigate("/unlock-vault", { replace: true });
      }
    } catch (error) {
      setError(String(error));
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="nv-page-enter flex min-h-screen w-full bg-(--color-background)">
      <ErrorPopup
        open={error !== null}
        message={error ?? ""}
        onClose={() => setError(null)}
      />
      <section className="mx-auto w-full max-w-6xl p-5 sm:p-6 xl:p-8">
        <div className="grid gap-8 xl:grid-cols-[1.05fr_0.95fr]">
          <div className="nv-fade-up nv-stagger-1 rounded-4xl border border-(--color-border-strong) bg-(--color-surface) p-8 shadow-[0_28px_60px_rgba(17,24,39,0.08)]">
            <div className="flex flex-col gap-6 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <p className="text-[13px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                  Vault dashboard
                </p>
                <h1 className="mt-3 text-4xl font-semibold text-(--color-text-primary)">
                  Your Vaults
                </h1>
                <p className="mt-3 max-w-2xl text-[16px] text-(--color-text-secondary)">
                  Switch between Vaults, open new folders, or lock the current
                  session when you’re done.
                </p>
              </div>
              <div className="rounded-3xl border border-(--color-border) bg-(--color-surface-muted) px-5 py-4 text-center">
                <p className="text-[12px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                  Saved Vaults
                </p>
                <p className="mt-2 text-[24px] font-semibold text-(--color-text-primary)">
                  {vaults.length}
                </p>
                <p className="text-[13px] text-(--color-text-secondary)">
                  {vaults.length === 1 ? "vault available" : "vaults available"}
                </p>
              </div>
            </div>
            <div className="mt-8 rounded-[28px] border border-(--color-border) bg-(--color-surface-muted) p-6">
              <div className="flex flex-col gap-5 sm:flex-row sm:items-center sm:justify-between">
                <div>
                  <p className="text-[13px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                    Current Vault
                  </p>
                  <h2 className="mt-3 text-[24px] font-semibold text-(--color-text-primary)">
                    {openVault?.name ?? "No vault open"}
                  </h2>
                </div>
                <div className="inline-flex items-center gap-2 rounded-full bg-(--color-background) px-3 py-2 text-[13px] text-(--color-text-secondary)">
                  <Lock className="h-4 w-4" />
                  {openVault ? "Unlocked" : "Locked"}
                </div>
              </div>
              <p className="mt-4 text-[14px] text-(--color-text-secondary) break-all">
                {openVault?.path ||
                  "Open a Vault to keep it available until you lock the session or close the app."}
              </p>
              <div className="mt-6 grid gap-3 sm:grid-cols-2">
                <button
                  type="button"
                  className="rounded-2xl border border-(--color-border) bg-(--color-surface) px-5 py-4 text-[15px] font-medium text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-60"
                  onClick={handleOpenVault}
                  disabled={loading}
                >
                  Open another Vault
                </button>
                <button
                  type="button"
                  className="rounded-2xl bg-(--color-accent) px-5 py-4 text-[15px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active) disabled:opacity-60"
                  onClick={handleLock}
                  disabled={loading}
                >
                  Lock Vault
                </button>
              </div>
            </div>
          </div>
          <div className="nv-fade-up nv-stagger-2 rounded-4xl border border-(--color-border-strong) bg-(--color-surface) p-8 shadow-[0_28px_60px_rgba(17,24,39,0.08)]">
            <div className="mb-6">
              <p className="text-[13px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                Vault library
              </p>
              <h2 className="mt-3 text-2xl font-semibold text-(--color-text-primary)">
                Saved Vaults
              </h2>
              <p className="mt-3 text-[15px] text-(--color-text-secondary)">
                Open any saved Vault below, or add another one to keep it within
                reach for the current session.
              </p>
            </div>
            <div className="space-y-3">
              {vaults.length === 0 ? (
                <div className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) p-5 text-[15px] text-(--color-text-secondary)">
                  No saved Vaults yet. Open or create one to begin.
                </div>
              ) : (
                vaults.map((vault) => (
                  <div
                    key={vault.id}
                    className={`flex flex-col gap-3 rounded-2xl border px-4 py-4 transition hover:border-(--color-accent) sm:flex-row sm:items-center sm:justify-between ${
                      vault.id === openVault?.id
                        ? "border-(--color-accent) bg-(--color-accent-soft)"
                        : "border-(--color-border) bg-(--color-surface-muted)"
                    }`}
                  >
                    <div className="min-w-0">
                      <p className="text-[15px] font-semibold text-(--color-text-primary) truncate">
                        {vault.name}
                      </p>
                      <p className="mt-1 text-[13px] text-(--color-text-secondary) truncate">
                        {vault.path}
                      </p>
                    </div>
                    <div className="flex items-center gap-3">
                      {vault.id === openVault?.id && (
                        <span className="rounded-full bg-(--color-background) px-3 py-1 text-[13px] text-(--color-text-secondary)">
                          Open
                        </span>
                      )}
                      <button
                        type="button"
                        className="inline-flex items-center gap-2 rounded-2xl border border-(--color-border) bg-(--color-surface) px-4 py-2 text-[14px] text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                        onClick={() => handleSwitchVault(vault)}
                        disabled={loading}
                      >
                        <FolderOpen className="h-4 w-4 text-(--color-accent)" />
                        Open
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
            <div className="mt-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <button
                type="button"
                className="rounded-2xl border border-(--color-border) bg-(--color-surface) px-5 py-4 text-[15px] font-medium text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                onClick={() => navigate("/create-vault")}
                disabled={loading}
              >
                <Plus className="h-4 w-4" />
                Create Vault
              </button>
              <button
                type="button"
                className="rounded-2xl border border-(--color-border) bg-(--color-surface) px-5 py-4 text-[15px] font-medium text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                onClick={handleOpenVault}
                disabled={loading}
              >
                Open Vault...
              </button>
            </div>
          </div>
        </div>
        {passwordPromptVault && (
          <div
            className="nv-fade-up fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4"
            role="dialog"
            aria-modal="true"
            aria-labelledby="password-prompt-title"
          >
            <div className="w-full max-w-md rounded-2xl border border-(--color-border-strong) bg-(--color-surface) p-6 shadow-xl">
              <h2
                id="password-prompt-title"
                className="text-[18px] font-semibold text-(--color-text-primary)"
              >
                Unlock Vault
              </h2>
              <p className="mt-2 text-[15px] text-(--color-text-secondary)">
                Enter the password for {passwordPromptVault.name}
              </p>
              <form
                className="mt-5 flex flex-col gap-4"
                onSubmit={handlePasswordPromptSubmit}
              >
                <label
                  className="text-[15px] text-(--color-text-secondary)"
                  htmlFor="vault-password-prompt"
                >
                  Password
                </label>
                <input
                  id="vault-password-prompt"
                  type="password"
                  value={passwordPrompt}
                  onChange={(event) => setPasswordPrompt(event.target.value)}
                  disabled={passwordPromptLoading}
                  className="h-10 w-full rounded-[9px] border border-(--color-border) bg-(--color-surface-muted) px-3 text-[15px] text-(--color-text-primary) outline-none transition focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)] disabled:opacity-60"
                />
                {passwordPromptError && (
                  <p className="text-[14px] text-(--color-danger)">
                    {passwordPromptError}
                  </p>
                )}
                <div className="mt-2 flex justify-end gap-2">
                  <button
                    type="button"
                    className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-4 py-2 text-[15px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-50"
                    onClick={handlePasswordPromptCancel}
                    disabled={passwordPromptLoading}
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    className="rounded-2xl bg-(--color-accent) px-4 py-2 text-[15px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active) disabled:cursor-not-allowed disabled:opacity-60"
                    disabled={passwordPromptLoading}
                  >
                    {passwordPromptLoading ? "Unlocking..." : "Unlock"}
                  </button>
                </div>
              </form>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}

export default Vault;
