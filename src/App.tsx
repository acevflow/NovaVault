import { BrowserRouter, Routes, Route } from "react-router-dom";

import Welcome from "./pages/Welcome";
import GetStarted from "./pages/GetStarted";
import CreateVault from "./pages/CreateVault";
import UnlockVault from "./pages/UnlockVault";
import Vault from "./pages/Vault";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Welcome />} />
        <Route path="/get-started" element={<GetStarted />} />
        <Route path="/create-vault" element={<CreateVault />} />
        <Route path="/unlock-vault" element={<UnlockVault />} />
        <Route path="/vault" element={<Vault />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
