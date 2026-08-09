import { useNavigate } from "react-router-dom";
import Brand from "../components/Brand";

function Welcome() {
  const navigate = useNavigate();

  return (
    <main className="flex min-h-screen w-full flex-col bg-[var(--color-background)] lg:flex-row">
      <section className="flex flex-1 items-center justify-center p-8">
        <Brand />
      </section>
      <section className="flex flex-1 flex-col items-center justify-center gap-4 p-8">
        <button
          type="button"
          className="h-[50px] w-[200px] rounded-[16px] bg-[var(--color-accent)] px-4 text-[20px] font-semibold text-[var(--color-text-on-accent)] transition hover:-translate-y-1 hover:shadow-[0_4px_6px_rgba(0,0,0,0.1)] active:translate-y-0 active:shadow-[0_2px_3px_rgba(0,0,0,0.1)]"
          onClick={() => navigate("/create-vault")}
        >
          Create Vault
        </button>
        <button
          className="w-[200px] bg-transparent text-[16px] font-medium text-[var(--color-link)] transition hover:opacity-70 active:opacity-50"
          type="button"
        >
          Already have a Vault?
        </button>
      </section>
    </main>
  );
}

export default Welcome;
