import logo from "../assets/images/logo.png";

function CheckingVault() {
  return (
    <main className="flex min-h-screen w-full items-center justify-center bg-(--color-background) px-6 py-6">
      <section
        className="nv-page-enter flex w-full max-w-105 flex-col items-center justify-center rounded-3xl bg-(--color-surface) px-8 py-9 text-center shadow-[0_12px_40px_rgba(0,0,0,0.06)] sm:px-10 sm:py-10"
        role="status"
        aria-live="polite"
      >
        <div className="relative mb-5 flex h-24 w-24 items-center justify-center">
          <div
            className="absolute inset-0 animate-spin rounded-full border-4 border-(--color-accent-soft) border-t-(--color-accent)"
            aria-hidden="true"
          />
          <img src={logo} alt="" className="h-16 w-16 object-contain" />
        </div>
        <h1 className="nv-fade-up text-xl font-semibold text-(--color-text-primary) sm:text-2xl">
          Checking your Vaults
        </h1>
        <p className="nv-fade-up nv-stagger-1 mt-2 max-w-sm text-[14px] leading-relaxed text-(--color-text-secondary) sm:text-[15px]">
          Looking for your most recently used Vault...
        </p>
      </section>
    </main>
  );
}

export default CheckingVault;
