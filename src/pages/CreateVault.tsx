import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Eye, EyeOff } from "lucide-react";
import { useNavigate } from "react-router-dom";
import Brand from "../components/Brand";
import "../styles/create-vault.css";

function CreateVault() {
  const navigate = useNavigate();

  const handleBrowse = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (selected) {
      setStorageLocation(selected);
    }
  };
  const [storageLocation, setStorageLocation] = useState("");
  const [passwordProtection, setPasswordProtection] = useState(true);
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);

  return (
    <main className="create-vault">
      <section className="create-vault-brand">
        <Brand />
      </section>
      <section className="create-vault-content">
        <header className="create-vault-header">
          <h2 className="create-vault-header-title">Create Vault</h2>
          <p className="create-vault-header-subtitle">
            Create your private storage locally.
          </p>
        </header>
        <form className="create-vault-form">
          <div className="create-vault-form-field">
            <label htmlFor="vault-name">Vault name</label>
            <input
              id="vault-name"
              name="vaultName"
              className="create-vault-form-input"
              type="text"
              placeholder="Personal Vault"
            />
          </div>
          <div className="create-vault-form-field">
            <label htmlFor="storage-location">Storage location</label>
            <div className="create-vault-storage-location">
              <input
                id="storage-location"
                name="storageLocation"
                className="create-vault-form-input create-vault-storage-input"
                type="text"
                value={storageLocation}
                readOnly
              />
              <button
                className="create-vault-browse-button"
                type="button"
                onClick={handleBrowse}
              >
                Browse
              </button>
            </div>
          </div>
          <fieldset className="password-protection">
            <legend>Protect with a password</legend>
            <div className="create-vault-password-options">
              <label>
                <input
                  className="create-vault-password-radio"
                  type="radio"
                  name="passwordProtection"
                  value="yes"
                  checked={passwordProtection}
                  onChange={() => setPasswordProtection(true)}
                />
                Yes
              </label>
              <label>
                <input
                  className="create-vault-password-radio"
                  type="radio"
                  name="passwordProtection"
                  value="no"
                  checked={!passwordProtection}
                  onChange={() => setPasswordProtection(false)}
                />
                No
              </label>
            </div>
          </fieldset>
          {passwordProtection && (
            <>
              <div className="create-vault-form-field">
                <label htmlFor="password">Password</label>
                <div className="create-vault-password-input">
                  <input
                    id="password"
                    name="password"
                    className="create-vault-form-input create-vault-password-field"
                    type={showPassword ? "text" : "password"}
                  />
                  <button
                    className="create-vault-password-toggle"
                    type="button"
                    onClick={() => setShowPassword(!showPassword)}
                  >
                    {showPassword ? (
                      <EyeOff className="create-vault-password-icon" />
                    ) : (
                      <Eye className="create-vault-password-icon" />
                    )}
                  </button>
                </div>
              </div>
              <div className="create-vault-form-field">
                <label htmlFor="confirm-password">Confirm password</label>
                <div className="create-vault-password-input">
                  <input
                    id="confirm-password"
                    name="confirmPassword"
                    className="create-vault-form-input create-vault-password-field"
                    type={showConfirmPassword ? "text" : "password"}
                  />
                  <button
                    className="create-vault-password-toggle"
                    type="button"
                    onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                  >
                    {showConfirmPassword ? (
                      <EyeOff className="create-vault-password-icon" />
                    ) : (
                      <Eye className="create-vault-password-icon" />
                    )}
                  </button>
                </div>
              </div>
            </>
          )}
          <div className="create-vault-actions">
            <button
              className="create-vault-secondary-button"
              type="button"
              onClick={() => navigate("/")}
            >
              Back
            </button>
            <button className="create-vault-primary-button" type="submit">
              Create Vault
            </button>
          </div>
        </form>
      </section>
    </main>
  );
}

export default CreateVault;
