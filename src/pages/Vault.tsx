import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Plus, FolderOpen, Lock, Unlock, FolderPlus, FilePlus } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import ErrorPopup from "../components/ErrorPopup";

interface SavedVault {
  id: string;
  name: string;
  path: string;
}

interface VaultFolder {
  id: string;
  parent_id: string | null;
  name: string;
  created_at: string;
  updated_at: string;
}

interface VaultFile {
  id: string;
  name: string;
  folder_id: string | null;
  object_path: string;
  created_at: string;
  updated_at: string;
}

interface VaultNote {
  id: string;
  title: string;
  content: string;
  created_at: string;
  updated_at: string;
}

interface VaultPassword {
  id: string;
  title: string;
  username?: string;
  notes?: string;
  created_at: string;
  updated_at: string;
}

interface VaultContents {
  folders: VaultFolder[];
  files: VaultFile[];
  notes: VaultNote[];
  passwords: VaultPassword[];
}

function Vault() {
  const [error, setError] = useState<string | null>(null);

  const navigate = useNavigate();

  const [vaults, setVaults] = useState<SavedVault[]>([]);
  const [openVault, setOpenVault] = useState<SavedVault | null>(null);
  const [vaultContents, setVaultContents] = useState<VaultContents | null>(
    null,
  );
  const [selectedTab, setSelectedTab] = useState<
    "files" | "notes" | "passwords"
  >("files");
  const [passwordPromptVault, setPasswordPromptVault] =
    useState<SavedVault | null>(null);
  const [passwordPrompt, setPasswordPrompt] = useState("");
  const [passwordPromptError, setPasswordPromptError] = useState<string | null>(
    null,
  );
  const [passwordPromptLoading, setPasswordPromptLoading] = useState(false);
  const [creatingFolder, setCreatingFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
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
            await loadVaultContents(currentVault.path);
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
      await loadVaultContents(vault.path);
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
      await loadVaultContents(passwordPromptVault.path);
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

  const loadVaultContents = async (vaultPath: string) => {
    setLoading(true);
    try {
      const contents = await invoke<VaultContents>("list_vault_contents", {
        vaultPath,
      });
      setVaultContents(contents);
    } catch (error) {
      setError(String(error));
    } finally {
      setLoading(false);
    }
  };

  const handleSelectVault = async () => {
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
        await loadVaultContents(path);
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

  const handleAddFileToVault = async () => {
    if (!openVault || loading) {
      return;
    }

    const selected = await open({
      multiple: false,
      directory: false,
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
      await invoke("add_file", {
        vaultPath: openVault.path,
        sourcePath: path,
        folderId: null,
      });
      await loadVaultContents(openVault.path);
    } catch (error) {
      setError(String(error));
    } finally {
      setLoading(false);
    }
  };

  const handleCreateFolder = () => {
    if (!openVault || loading) {
      return;
    }
    setNewFolderName("");
    setCreatingFolder(true);
  };

  const handleConfirmCreateFolder = async () => {
    if (!openVault || !newFolderName.trim()) {
      setError("Enter a valid folder name.");
      return;
    }

    setLoading(true);

    try {
      await invoke("create_folder", {
        vaultPath: openVault.path,
        parentId: null,
        name: newFolderName.trim(),
      });
      setCreatingFolder(false);
      await loadVaultContents(openVault.path);
    } catch (error) {
      setError(String(error));
    } finally {
      setLoading(false);
    }
  };

  const handleCancelCreateFolder = () => {
    setCreatingFolder(false);
    setNewFolderName("");
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
          await loadVaultContents(nextVaultPath);
        }
      } else {
        setVaultContents(null);
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
        <div className="grid gap-6 xl:grid-cols-[310px_minmax(0,1fr)]">
          <aside className="nv-fade-up nv-stagger-1 rounded-[24px] border border-(--color-border) bg-(--color-surface) p-5">
            <div className="mb-5">
              <p className="text-[12px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                Vaults
              </p>
              <h2 className="mt-2 text-[30px] font-semibold text-(--color-text-primary)">
                Saved Vaults
              </h2>
            </div>

            <div className="space-y-3">
              {vaults.length === 0 ? (
                <div className="rounded-[18px] border border-(--color-border) bg-(--color-surface-muted) p-4 text-[15px] text-(--color-text-secondary)">
                  No Vaults saved yet.
                </div>
              ) : (
                vaults.map((vault) => (
                  <div
                    key={vault.id}
                    className={`rounded-[18px] border p-3 transition ${
                      vault.id === openVault?.id
                        ? "border-(--color-accent) bg-(--color-accent-soft)"
                        : "border-(--color-border) bg-(--color-surface-muted)"
                    }`}
                  >
                    <div className="flex items-start gap-3">
                      <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-(--color-surface)">
                        {vault.id === openVault?.id ? (
                          <Unlock className="h-4 w-4 text-(--color-accent)" />
                        ) : (
                          <Lock className="h-4 w-4 text-(--color-text-secondary)" />
                        )}
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-[15px] font-semibold text-(--color-text-primary)">
                          {vault.name}
                        </p>
                        <p className="mt-1 truncate text-[12px] text-(--color-text-secondary)">
                          {vault.path}
                        </p>
                      </div>
                    </div>

                    <div className="mt-3 flex items-center justify-between gap-2">
                      <span
                        className={`rounded-full px-2.5 py-1 text-[11px] ${
                          vault.id === openVault?.id
                            ? "bg-(--color-surface) text-(--color-accent)"
                            : "bg-(--color-background) text-(--color-text-secondary)"
                        }`}
                      >
                        {vault.id === openVault?.id ? "Open" : "Closed"}
                      </span>
                      <button
                        type="button"
                        className="rounded-xl border border-(--color-border) bg-(--color-surface) px-3 py-1.5 text-[13px] text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                        onClick={() => handleSwitchVault(vault)}
                        disabled={loading}
                      >
                        Open
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>

            <div className="mt-5 space-y-3">
              <button
                type="button"
                className="w-full rounded-2xl border border-(--color-border) bg-(--color-surface) px-4 py-3 text-[15px] font-medium text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                onClick={() => navigate("/create-vault")}
                disabled={loading}
              >
                <Plus className="mr-2 inline h-4 w-4" />
                Create Vault
              </button>
              <button
                type="button"
                className="w-full rounded-2xl border border-(--color-border) bg-(--color-surface) px-4 py-3 text-[15px] font-medium text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                onClick={handleSelectVault}
                disabled={loading}
              >
                <FolderOpen className="mr-2 inline h-4 w-4" />
                Open Vault...
              </button>
            </div>
          </aside>

          <section className="nv-fade-up nv-stagger-2 rounded-[24px] border border-(--color-border) bg-(--color-surface) p-5 sm:p-6">
            <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <p className="text-[12px] uppercase tracking-[0.24em] text-(--color-text-secondary)">
                  Active Vault
                </p>
                <h1 className="mt-2 text-[30px] font-semibold text-(--color-text-primary)">
                  {openVault?.name ?? "Select a Vault"}
                </h1>
              </div>
              <div className="inline-flex items-center gap-2 rounded-full bg-(--color-surface-muted) px-3 py-2 text-[13px] text-(--color-text-secondary)">
                {openVault ? (
                  <>
                    <Unlock className="h-4 w-4 text-(--color-accent)" />
                    Unlocked
                  </>
                ) : (
                  <>
                    <Lock className="h-4 w-4" />
                    Locked
                  </>
                )}
              </div>
            </div>

            <p className="mt-4 max-w-2xl break-all text-[15px] leading-relaxed text-(--color-text-secondary)">
              {openVault?.path ?? "Select a Vault from the list to manage its content."}
            </p>

            {openVault && (
              <div className="mt-5 flex flex-wrap gap-3">
                <button
                  type="button"
                  className="inline-flex items-center gap-2 rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-4 py-2 text-[14px] text-(--color-text-primary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
                  onClick={handleLock}
                  disabled={loading}
                >
                  <Lock className="h-4 w-4" />
                  Lock Vault
                </button>
              </div>
            )}

            {openVault && vaultContents ? (
              <div className="mt-8 space-y-6">
                <div className="rounded-[20px] border border-(--color-border) bg-(--color-surface-muted) p-4">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div className="flex flex-wrap items-center gap-2">
                      {(["files", "notes", "passwords"] as const).map((tab) => (
                        <button
                          key={tab}
                          type="button"
                          className={`rounded-full px-3.5 py-2 text-[14px] transition ${
                            selectedTab === tab
                              ? "bg-(--color-accent-soft) text-(--color-accent)"
                              : "bg-(--color-surface) text-(--color-text-secondary) hover:bg-(--color-surface-hover)"
                          }`}
                          onClick={() => setSelectedTab(tab)}
                        >
                          {tab.charAt(0).toUpperCase() + tab.slice(1)}
                        </button>
                      ))}
                    </div>

                    <div className="flex flex-wrap gap-3">
                      <button
                        type="button"
                        className="inline-flex items-center gap-2 rounded-2xl border border-(--color-border) bg-(--color-surface) px-3 py-2 text-[14px] text-(--color-text-primary) transition hover:bg-(--color-surface-hover)"
                        onClick={handleAddFileToVault}
                        disabled={loading}
                      >
                        <FilePlus className="h-4 w-4" />
                        Add File
                      </button>
                      <button
                        type="button"
                        className="inline-flex items-center gap-2 rounded-2xl border border-(--color-border) bg-(--color-surface) px-3 py-2 text-[14px] text-(--color-text-primary) transition hover:bg-(--color-surface-hover)"
                        onClick={handleCreateFolder}
                        disabled={loading}
                      >
                        <FolderPlus className="h-4 w-4" />
                        New Folder
                      </button>
                    </div>
                  </div>
                </div>

                <div className="space-y-4">
                  {selectedTab === "files" && (
                    <>
                      {vaultContents.folders.length > 0 && (
                        <div className="rounded-[18px] border border-(--color-border) bg-(--color-surface-muted) p-4">
                          <p className="text-[15px] font-semibold text-(--color-text-primary)">
                            Folders
                          </p>
                          <div className="mt-4 grid gap-3">
                            {vaultContents.folders.map((folder) => (
                              <div
                                key={folder.id}
                                className="rounded-2xl border border-(--color-border) bg-(--color-surface) p-3"
                              >
                                <p className="text-[14px] font-medium text-(--color-text-primary)">
                                  {folder.name}
                                </p>
                                <p className="mt-1 text-[12px] text-(--color-text-secondary)">
                                  {folder.created_at}
                                </p>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      {vaultContents.files.length === 0 ? (
                        <div className="rounded-[18px] border border-(--color-border) bg-(--color-surface-muted) p-6 text-[15px] text-(--color-text-secondary)">
                          No files saved yet.
                        </div>
                      ) : (
                        <div className="grid gap-3">
                          {vaultContents.files.map((file) => (
                            <div
                              key={file.id}
                              className="rounded-[18px] border border-(--color-border) bg-(--color-surface) p-4"
                            >
                              <p className="text-[15px] font-semibold text-(--color-text-primary)">
                                {file.name}
                              </p>
                              <p className="mt-1 text-[13px] text-(--color-text-secondary)">
                                {file.object_path}
                              </p>
                            </div>
                          ))}
                        </div>
                      )}
                    </>
                  )}

                  {selectedTab === "notes" && (
                    <div className="grid gap-3">
                      {vaultContents.notes.length === 0 ? (
                        <div className="rounded-[18px] border border-(--color-border) bg-(--color-surface-muted) p-6 text-[15px] text-(--color-text-secondary)">
                          No notes saved yet.
                        </div>
                      ) : (
                        vaultContents.notes.map((note) => (
                          <div
                            key={note.id}
                            className="rounded-[18px] border border-(--color-border) bg-(--color-surface) p-4"
                          >
                            <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                              <p className="text-[15px] font-semibold text-(--color-text-primary)">
                                {note.title}
                              </p>
                              <p className="text-[13px] text-(--color-text-secondary)">
                                {note.created_at}
                              </p>
                            </div>
                            <p className="mt-3 text-[14px] text-(--color-text-secondary)">
                              {note.content}
                            </p>
                          </div>
                        ))
                      )}
                    </div>
                  )}

                  {selectedTab === "passwords" && (
                    <div className="grid gap-3">
                      {vaultContents.passwords.length === 0 ? (
                        <div className="rounded-[18px] border border-(--color-border) bg-(--color-surface-muted) p-6 text-[15px] text-(--color-text-secondary)">
                          No passwords saved yet.
                        </div>
                      ) : (
                        vaultContents.passwords.map((password) => (
                          <div
                            key={password.id}
                            className="rounded-[18px] border border-(--color-border) bg-(--color-surface) p-4"
                          >
                            <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                              <p className="text-[15px] font-semibold text-(--color-text-primary)">
                                {password.title}
                              </p>
                              <p className="text-[13px] text-(--color-text-secondary)">
                                {password.username ?? "No user"}
                              </p>
                            </div>
                            <p className="mt-3 text-[14px] text-(--color-text-secondary)">
                              {password.notes ?? "No notes"}
                            </p>
                          </div>
                        ))
                      )}
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <div className="mt-8 rounded-[18px] border border-(--color-border) bg-(--color-surface-muted) p-10 text-[15px] text-(--color-text-secondary)">
                Select a Vault from the list to start managing its content.
              </div>
            )}
          </section>
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

        {creatingFolder && (
          <div
            className="nv-fade-up fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4"
            role="dialog"
            aria-modal="true"
            aria-labelledby="create-folder-title"
          >
            <div className="w-full max-w-md rounded-2xl border border-(--color-border-strong) bg-(--color-surface) p-6 shadow-xl">
              <h2
                id="create-folder-title"
                className="text-[18px] font-semibold text-(--color-text-primary)"
              >
                Create Folder
              </h2>
              <p className="mt-2 text-[15px] text-(--color-text-secondary)">
                Enter a name for the new folder inside the current Vault.
              </p>
              <div className="mt-5 flex flex-col gap-4">
                <label
                  className="text-[15px] text-(--color-text-secondary)"
                  htmlFor="new-folder-name"
                >
                  Folder Name
                </label>
                <input
                  id="new-folder-name"
                  type="text"
                  value={newFolderName}
                  onChange={(event) => setNewFolderName(event.target.value)}
                  disabled={loading}
                  className="h-12 w-full rounded-[11px] border border-(--color-border) bg-(--color-surface-muted) px-4 text-[15px] text-(--color-text-primary) outline-none transition focus:border-(--color-accent) focus:shadow-[0_0_0_2px_rgba(37,99,235,0.15)] disabled:opacity-60"
                />
                <div className="flex justify-end gap-2">
                  <button
                    type="button"
                    className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-4 py-2 text-[15px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active) disabled:opacity-50"
                    onClick={handleCancelCreateFolder}
                    disabled={loading}
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    className="rounded-2xl bg-(--color-accent) px-4 py-2 text-[15px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active) disabled:cursor-not-allowed disabled:opacity-60"
                    onClick={handleConfirmCreateFolder}
                    disabled={loading || !newFolderName.trim()}
                  >
                    Create Folder
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}

export default Vault;
