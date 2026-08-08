import { useNavigate } from "react-router-dom";
import Brand from "../components/Brand";
import "../styles/welcome.css";

function Welcome() {
  const navigate = useNavigate();

  return (
    <main className="welcome">
      <section className="welcome-brand">
        <Brand />
      </section>
      <section className="welcome-content">
        <button
          type="button"
          className="welcome-primary-button"
          onClick={() => navigate("/create-vault")}
        >
          Create Vault
        </button>
        <button className="welcome-secondary-button" type="button">
          Already have a Vault?
        </button>
      </section>
    </main>
  );
}

export default Welcome;
