import "../styles/brand.css";
import logo from "../assets/images/logo.png";

function Brand() {
  return (
    <div className="brand">
      <img className="brand-logo" src={logo} alt="NovaVault logo" />
      <h1 className="brand-title">NovaVault</h1>
      <p className="brand-subtitle">Your data. Your vault. Your control.</p>
    </div>
  );
}

export default Brand;
