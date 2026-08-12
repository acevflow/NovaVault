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
    if (loading || vault.id === openVault?.id) {
      setMenuOpen(false);
      return;
    }

    setLoading(true);

    try {
      await invoke("close_vault");

      await invoke("open_vault", {
        vaultPath: vault.path,
      });

      setOpenVault(vault);

      const updatedVaults = await invoke<SavedVault[]>("get_saved_vaults");

      setVaults(updatedVaults);
      setMenuOpen(false);
    } catch (error) {
      setError(String(error));
    } finally {
      setLoading(false);
    }
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
              Browse your Vault files and folders.
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
          <p className="text-(--color-text-muted)">Your Vault is empty.</p>
        </div>
      </section>
    </main>
  );
}

export default Vault;
