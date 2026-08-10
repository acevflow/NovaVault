function Vault() {
  return (
    <main className="flex min-h-screen w-full bg-(--color-background)">
      <section className="flex w-full flex-col p-8">
        <header className="mb-8">
          <h1 className="text-4xl font-semibold text-(--color-text-primary)">
            Files
          </h1>
          <p className="mt-2 text-[16px] text-(--color-text-secondary)">
            Browse your Vault files and folders.
          </p>
        </header>
        <div className="flex flex-1 items-center justify-center">
          <p className="text-(--color-text-muted)">Your Vault is empty.</p>
        </div>
      </section>
    </main>
  );
}

export default Vault;
