import { ArrowRight } from "lucide-react";
import { useNavigate } from "react-router-dom";
import Brand from "../components/Brand";

function Welcome() {
  const navigate = useNavigate();

  return (
    <main className="flex min-h-screen w-full flex-row bg-(--color-background)">
      <section className="nv-page-enter flex min-w-0 flex-1 items-center justify-center p-8 max-[1100px]:p-5">
        <Brand />
      </section>
      <section className="flex min-w-0 flex-1 flex-col items-center justify-center gap-4 p-8 max-[1100px]:gap-3 max-[1100px]:p-5">
        <button
          type="button"
          className="nv-fade-up nv-stagger-2 flex h-12.5 w-50 items-center justify-center gap-2 rounded-2xl bg-(--color-accent) px-4 text-[20px] font-semibold text-(--color-text-on-accent) transition hover:-translate-y-1 hover:shadow-[0_4px_6px_rgba(0,0,0,0.1)] active:translate-y-0 active:shadow-[0_2px_3px_rgba(0,0,0,0.1)]"
          onClick={() => navigate("/get-started")}
        >
          Get Started
          <ArrowRight className="h-5 w-5" />
        </button>
        <button
          className="nv-fade-up nv-stagger-3 w-50 bg-transparent text-[16px] font-medium text-(--color-link) transition hover:opacity-70 active:opacity-50"
          type="button"
        >
          Already have a Vault?
        </button>
      </section>
    </main>
  );
}

export default Welcome;
