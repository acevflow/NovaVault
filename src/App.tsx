import { BrowserRouter, Routes, Route } from "react-router-dom";

import Welcome from "./pages/Welcome";
import CreateVault from "./pages/CreateVault";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Welcome />} />
        <Route path="/create-vault" element={<CreateVault />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
