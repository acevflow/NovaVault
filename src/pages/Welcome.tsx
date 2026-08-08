import "./Welcome.css";

import logo from "../assets/images/logo.png";

function Welcome() {
  return (
    <main className="welcome">
      <section className="welcome-content">
        <img className="welcome-logo" src={logo} alt="NovaVault logo" />
        <h1 className="welcome-title">NovaVault</h1>
        <p className="welcome-subtitle">Your data. Your vault. Your control.</p>
      </section>
      <section className="welcome-actions">
        <button className="welcome-primary-button">Create Vault</button>
        <button className="welcome-secondary-button">
          Already have a Vault?
        </button>
      </section>
    </main>
  );
}

export default Welcome;
