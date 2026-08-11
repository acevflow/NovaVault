import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowRight } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import Brand from "../components/Brand";
import CheckingVault from "../components/CheckingVault";

function Welcome() {
  const navigate = useNavigate();

  const [checkingVault, setCheckingVault] = useState(true);

  useEffect(() => {
    let cancelled = false;

    const checkOpenVault = async () => {
      try {
        const openVault = await invoke<string | null>("get_open_vault");

        if (cancelled) {
          return;
        }

        if (openVault) {
          navigate("/vault", { replace: true });
          return;
        }
      } catch (error) {
        console.error("Failed to check for an open Vault:", error);
      }

      if (!cancelled) {
        setCheckingVault(false);
      }
    };

    checkOpenVault();

    return () => {
      cancelled = true;
    };
  }, [navigate]);

  const handleOpenVault = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (!selected) {
      return;
    }

    try {
      await invoke("open_vault", {
        vaultPath: selected,
      });

      navigate("/vault");
    } catch (error) {
      console.error("Failed to open Vault:", error);
    }
  };

  if (checkingVault) {
    return <CheckingVault />;
  }

  return (
    <main className="flex min-h-screen w-full flex-row bg-(--color-background)">
      <section className="nv-page-enter flex min-w-0 flex-1 items-center justify-center p-8 max-[1100px]:p-5">
        <Brand />
      </section>
      <section className="flex min-w-0 flex-1 flex-col items-center justify-center gap-4 p-8 max-[1100px]:gap-3 max-[1100px]:p-5">
        <button
          type="button"
          className="nv-fade-up nv-stagger-2 flex h-12.5 w-50 items-center justify-center gap-2 rounded-2xl bg-(--color-accent) px-4 text-[20px] font-semibold text-(--color-text-on-accent) transition hover:bg-(--color-accent-hover) active:bg-(--color-accent-active)"
          onClick={() => navigate("/get-started")}
        >
          Get Started
          <ArrowRight className="h-5 w-5" />
        </button>
        <button
          className="nv-fade-up nv-stagger-3 w-50 bg-transparent text-[16px] font-medium text-(--color-link) transition hover:text-(--color-link-hover) hover:opacity-70 active:text-(--color-link-active) active:opacity-50"
          type="button"
          onClick={handleOpenVault}
        >
          Already have a Vault?
        </button>
      </section>
    </main>
  );
}

export default Welcome;
