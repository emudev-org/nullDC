import { AppLayout } from "./app/layout/AppLayout";
import { HomePage } from "./app/layout/HomePage";
import { DocsPage } from "./app/layout/DocsPage";
import { Sh4SimPage } from "./app/layout/Sh4SimPage";
import { DspPlaygroundPage } from "./app/layout/DspPlaygroundPage";
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";

const App = () => {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/sh4-sim" element={<Sh4SimPage />} />
        <Route path="/dsp-playground/*" element={<DspPlaygroundPage />} />
        <Route path="/docs" element={<DocsPage />} />
        <Route path="/:tab" element={<AppLayout />} />
        <Route path="/:tab/:subtab" element={<AppLayout />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </HashRouter>
  );
};

export default App;
