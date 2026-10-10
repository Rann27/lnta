import React from "react";
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";
import { Shell } from "./Shell";
import { ProjectsPage } from "../features/projects";

export const AppRouter: React.FC = () => {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<Shell />}>
          <Route index element={<ProjectsPage />} />
          <Route
            path="workspace"
            element={
              <div style={{ padding: "var(--space-6)" }}>
                <h2>Workspace</h2>
              </div>
            }
          />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
};
