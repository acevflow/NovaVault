import { useNavigate } from "react-router-dom";
import "../styles/welcome.css";
import logo from "../assets/images/logo.png";

function Welcome() {
  const navigate = useNavigate();
  
  return (
    <main className="welcome">
      <section className="welcome-content">
        <img className="welcome-logo" src={logo} alt="NovaVault logo" />
        <h1 className="welcome-title">NovaVault</h1>
        <p className="welcome-subtitle">Your data. Your vault. Your control.</p>
      </section>
      <section className="welcome-actions">
        <button type="button" className="welcome-primary-button" onClick={() => navigate("/create-vault")}>Create Vault</button>
        <button type="button" className="welcome-secondary-button">
          Already have a Vault?
        </button>
      </section>
    </main>
  );
}

export default Welcome;
