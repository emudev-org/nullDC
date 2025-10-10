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
        <Route path="/workspace/sh4-debugger/:tab" element={<AppLayout workspaceId="sh4-debugger" />} />
        <Route path="/workspace/sh4-debugger/:tab/:subtab" element={<AppLayout workspaceId="sh4-debugger" />} />

        <Route path="/workspace/arm7-debugger" element={<AppLayout workspaceId="arm7-debugger" />} />
        <Route path="/workspace/arm7-debugger/:tab" element={<AppLayout workspaceId="arm7-debugger" />} />
        <Route path="/workspace/arm7-debugger/:tab/:subtab" element={<AppLayout workspaceId="arm7-debugger" />} />

        <Route path="/workspace/dsp-debugger" element={<AppLayout workspaceId="dsp-debugger" />} />
        <Route path="/workspace/dsp-debugger/:tab" element={<AppLayout workspaceId="dsp-debugger" />} />
        <Route path="/workspace/dsp-debugger/:tab/:subtab" element={<AppLayout workspaceId="dsp-debugger" />} />

        <Route path="/workspace/custom-debugger" element={<AppLayout workspaceId="custom-debugger" />} />
        <Route path="/workspace/custom-debugger/:tab" element={<AppLayout workspaceId="custom-debugger" />} />
        <Route path="/workspace/custom-debugger/:tab/:subtab" element={<AppLayout workspaceId="custom-debugger" />} />

        <Route path="/workspace/sh4-sim" element={<Sh4SimPage />} />
        <Route path="/workspace/dsp-playground/*" element={<DspPlaygroundPage />} />
        <Route path="/workspace/clx2-ta-log-analyzer" element={<Clx2TaLogAnalyzerPage />} />
        <Route path="/workspace/clx2-core-log-analyzer" element={<Clx2CoreLogAnalyzerPage />} />

        {/* Redirects from old routes to new workspace routes */}
        <Route path="/sh4-sim" element={<Navigate to="/workspace/sh4-sim" replace />} />
        <Route path="/dsp-playground/*" element={<Navigate to="/workspace/dsp-playground" replace />} />
        <Route path="/clx2-ta-log-analyzer" element={<Navigate to="/workspace/clx2-ta-log-analyzer" replace />} />
        <Route path="/clx2-core-log-analyzer" element={<Navigate to="/workspace/clx2-core-log-analyzer" replace />} />
        <Route path="/events" element={<Navigate to="/workspace/custom-debugger/events" replace />} />

        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </HashRouter>
  );
};

export default App;
