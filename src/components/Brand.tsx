import logo from "../assets/images/logo.png";

function Brand() {
  return (
    <div className="flex flex-col items-center justify-center text-center">
      <img className="h-64 w-64" src={logo} alt="NovaVault logo" />
      <h1 className="mt-4 text-5xl font-semibold text-[var(--color-text-primary)]">
        NovaVault
      </h1>
      <p className="mt-2 text-lg text-[var(--color-text-secondary)]">
        Your data. Your vault. Your control.
      </p>
    </div>
  );
}

export default Brand;
