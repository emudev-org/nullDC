import { AppLayout } from "./app/layout/AppLayout";
import { HomePage } from "./app/layout/HomePage";
import { DocsPage } from "./app/layout/DocsPage";
import { Sh4SimPage } from "./app/layout/Sh4SimPage";
import { Clx2TaLogAnalyzerPage } from "./app/layout/Clx2TaLogAnalyzerPage";
import { Clx2CoreLogAnalyzerPage } from "./app/layout/Clx2CoreLogAnalyzerPage";
import { DspPlaygroundPage } from "./app/layout/DspPlaygroundPage";
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";

const App = () => {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/docs" element={<DocsPage />} />

        {/* Workspace routes - each has its own layout persistence */}
        <Route path="/workspace/sh4-debugger" element={<AppLayout workspaceId="sh4-debugger" />} />
        <Route path="/workspace/arm7-debugger" element={<AppLayout workspaceId="arm7-debugger" />} />
        <Route path="/workspace/audio-debugger" element={<AppLayout workspaceId="audio-debugger" />} />
        <Route path="/workspace/mixed-mode-debugger" element={<AppLayout workspaceId="mixed-mode-debugger" />} />
        
        <Route path="/tool/sh4-sim" element={<Sh4SimPage />} />
        <Route path="/tool/dsp-playground" element={<DspPlaygroundPage />} />
        <Route path="/tool/clx2-ta-log-analyzer" element={<Clx2TaLogAnalyzerPage />} />
        <Route path="/tool/clx2-core-log-analyzer" element={<Clx2CoreLogAnalyzerPage />} />

        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </HashRouter>
  );
};

export default App;
