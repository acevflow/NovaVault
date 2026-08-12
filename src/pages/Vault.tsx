import { useNavigate } from "react-router-dom";

function Vault() {
  const navigate = useNavigate();

  return (
    <main className="flex min-h-screen w-full bg-(--color-background)">
      <section className="flex w-full flex-col p-8">
        <header className="mb-8 flex items-start justify-between gap-4">
          <div>
            <h1 className="text-4xl font-semibold text-(--color-text-primary)">
              Files
            </h1>
            <p className="mt-2 text-[16px] text-(--color-text-secondary)">
              Browse your Vault files and folders.
            </p>
          </div>
          <button
            type="button"
            className="rounded-2xl bg-(--color-accent) px-4 py-2 text-[16px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active)"
            onClick={() => navigate("/create-vault")}
          >
            Create another Vault
          </button>
        </header>
        <div className="flex flex-1 items-center justify-center">
          <p className="text-(--color-text-muted)">Your Vault is empty.</p>
        </div>
      </section>
    </main>
  );
}

export default Vault;
