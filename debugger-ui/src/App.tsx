import { AppLayout } from "./app/layout/AppLayout";
import { HomePane } from "./app/layout/HomePane";
import { AboutPane } from "./app/layout/AboutPane";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";

const App = () => {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<HomePane />} />
        <Route path="/:tab" element={<AppLayout />} />
        <Route path="/about" element={<AboutPane />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
};

export default App;
