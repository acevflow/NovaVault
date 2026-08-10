import { Folder, KeyRound, NotebookPen, ShieldCheck } from "lucide-react";
import { useNavigate } from "react-router-dom";
import Brand from "../components/Brand";

function GetStarted() {
  const navigate = useNavigate();

  return (
    <main className="flex min-h-screen w-full flex-col bg-(--color-background) p-8">
      <section className="nv-page-enter mx-auto flex w-full max-w-4xl flex-1 flex-col items-center justify-center">
        <Brand />
        <div className="mt-10 w-full">
          <div className="grid gap-4 sm:grid-cols-2">
            <article className="nv-fade-up nv-stagger-1 flex gap-4 rounded-2xl bg-(--color-surface) p-5">
              <ShieldCheck className="mt-1 h-6 w-6 shrink-0 text-(--color-accent)" />
              <div>
                <h2 className="text-[18px] font-semibold text-(--color-text-primary)">
                  Private & Local
                </h2>
                <p className="mt-1 text-[15px] leading-normal text-(--color-text-secondary)">
                  Your data stays stored locally on your computer.
                </p>
              </div>
            </article>
            <article className="nv-fade-up nv-stagger-2 flex gap-4 rounded-2xl bg-(--color-surface) p-5">
              <Folder className="mt-1 h-6 w-6 shrink-0 text-(--color-accent)" />
              <div>
                <h2 className="text-[18px] font-semibold text-(--color-text-primary)">
                  Organized
                </h2>
                <p className="mt-1 text-[15px] leading-normal text-(--color-text-secondary)">
                  Keep your files and folders organized in Vaults.
                </p>
              </div>
            </article>
            <article className="nv-fade-up nv-stagger-3 flex gap-4 rounded-2xl bg-(--color-surface) p-5">
              <NotebookPen className="mt-1 h-6 w-6 shrink-0 text-(--color-accent)" />
              <div>
                <h2 className="text-[18px] font-semibold text-(--color-text-primary)">
                  Notes & Passwords
                </h2>
                <p className="mt-1 text-[15px] leading-normal text-(--color-text-secondary)">
                  Store notes and passwords alongside your files.
                </p>
              </div>
            </article>
            <article className="nv-fade-up nv-stagger-4 flex gap-4 rounded-2xl bg-(--color-surface) p-5">
              <KeyRound className="mt-1 h-6 w-6 shrink-0 text-(--color-accent)" />
              <div>
                <h2 className="text-[18px] font-semibold text-(--color-text-primary)">
                  Protected
                </h2>
                <p className="mt-1 text-[15px] leading-normal text-(--color-text-secondary)">
                  Protect your Vaults, folders and individual items.
                </p>
              </div>
            </article>
          </div>
        </div>
        <div className="nv-fade-up nv-stagger-4 mt-8 flex w-full justify-end gap-3">
          <button
            type="button"
            className="rounded-2xl border border-(--color-border) bg-(--color-surface-muted) px-4 py-2 text-[18px] text-(--color-text-secondary) transition hover:bg-(--color-surface-hover) active:bg-(--color-surface-active)"
            onClick={() => navigate("/")}
          >
            Back
          </button>
          <button
            type="button"
            className="flex items-center gap-2 rounded-2xl bg-(--color-accent) px-4 py-2 text-[18px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active)"
            onClick={() => navigate("/create-vault")}
          >
            Continue
            <span aria-hidden="true">→</span>
          </button>
        </div>
      </section>
    </main>
  );
}

export default GetStarted;