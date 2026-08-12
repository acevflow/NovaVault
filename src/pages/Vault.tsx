import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ChevronDown, Plus, FolderOpen, Lock } from "lucide-react";
import { useEffect, useRef, useState } from "react";
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
  const [menuOpen, setMenuOpen] = useState(false);
  const [passwordPromptVault, setPasswordPromptVault] =
    useState<SavedVault | null>(null);
  const [passwordPrompt, setPasswordPrompt] = useState("");
  const [passwordPromptError, setPasswordPromptError] = useState<string | null>(
    null,
  );
  const [passwordPromptLoading, setPasswordPromptLoading] = useState(false);
  const [loading, setLoading] = useState(false);

  const menuRef = useRef<HTMLDivElement>(null);

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

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
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
      setMenuOpen(false);
    } catch (error) {
      const message = String(error);

      if (message.includes("This Vault is protected by a password")) {
        setPasswordPromptError(null);
        setPasswordPrompt("");
        setPasswordPromptVault(vault);
        setMenuOpen(false);
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
      setMenuOpen(false);
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

    setLoading(true);

    try {
      await invoke("open_vault", {
        vaultPath: selected,
      });

      const updatedVaults = await invoke<SavedVault[]>("get_saved_vaults");

      setVaults(updatedVaults);

      const currentVault = updatedVaults.find(
        (vault) => vault.path === selected,
      );

      if (currentVault) {
        setOpenVault(currentVault);
      }

      setMenuOpen(false);
    } catch (error) {
      console.error("Failed to open Vault:", error);
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
      await invoke("close_vault");
    } catch (error) {
      setError(String(error));
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="flex min-h-screen w-full bg-(--color-background)">
      <ErrorPopup
        open={error !== null}
        message={error ?? ""}
        onClose={() => setError(null)}
      />
      <section className="flex w-full flex-col p-8">
        <header className="mb-8 flex items-start justify-between">
          <div>
            <h1 className="text-4xl font-semibold text-(--color-text-primary)">
              Files
            </h1>
            <p className="mt-2 text-[16px] text-(--color-text-secondary)">
              Browse your Vault files and folders
            </p>
          </div>
          <div ref={menuRef} className="relative">
            <button
              type="button"
              className="flex items-center gap-2 rounded-2xl border border-(--color-border) bg-(--color-surface) px-4 py-2 text-[16px] font-medium text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
              onClick={() => setMenuOpen(!menuOpen)}
              disabled={loading}
            >
              <span>{openVault?.name ?? "Vault"}</span>
              <ChevronDown className="h-4 w-4" />
            </button>
            {menuOpen && (
              <div className="absolute right-0 z-20 mt-2 w-64 rounded-2xl border border-(--color-border-strong) bg-(--color-surface) p-2 shadow-xl">
                {vaults.map((vault) => (
                  <button
                    key={vault.id}
                    type="button"
                    className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-[15px] text-(--color-text-primary) transition hover:bg-(--color-surface-hover)"
                    onClick={() => handleSwitchVault(vault)}
                  >
                    <FolderOpen className="h-4 w-4 shrink-0 text-(--color-accent)" />
                    <span className="min-w-0 flex-1 truncate">
                      {vault.name}
                    </span>
                    {vault.id === openVault?.id && (
                      <span className="text-(--color-accent)">✓</span>
                    )}
                  </button>
                ))}
                <div className="my-2 border-t border-(--color-border)" />
                <button
                  type="button"
                  className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-[15px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover)"
                  onClick={() => {
                    setMenuOpen(false);
                    navigate("/create-vault");
                  }}
                >
                  <Plus className="h-4 w-4" />
                  Create Vault
                </button>
                <button
                  type="button"
                  className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-[15px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover)"
                  onClick={handleOpenVault}
                >
                  <FolderOpen className="h-4 w-4" />
                  Open Vault...
                </button>
                <button
                  type="button"
                  className="mt-1 flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-[15px] text-(--color-danger) transition hover:bg-(--color-surface-hover)"
                  onClick={handleLock}
                >
                  <Lock className="h-4 w-4" />
                  Lock Vault
                </button>
              </div>
            )}
          </div>
        </header>
        <div className="flex flex-1 items-center justify-center">
          <p className="text-(--color-text-muted)">Your Vault is empty</p>
        </div>
      </section>

      {passwordPromptVault && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4"
          role="dialog"
          aria-modal="true"
          aria-labelledby="password-prompt-title"
        >
          <div className="w-full max-w-md rounded-[20px] border border-(--color-border-strong) bg-(--color-surface) p-6 shadow-xl">
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
    </main>
  );
}

export default Vault;
