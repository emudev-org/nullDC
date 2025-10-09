import { AppLayout } from "./app/layout/AppLayout";
import { HomePage } from "./app/layout/HomePage";
import { DocsPage } from "./app/layout/DocsPage";
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";

const App = () => {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/docs" element={<DocsPage />} />
        <Route path="/:tab" element={<AppLayout />} />
        <Route path="/:tab/:subtab" element={<AppLayout />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </HashRouter>
  );
};

export default App;
