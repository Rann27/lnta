import React from "react";
import { HashRouter, Routes, Route, Navigate } from "react-router-dom";
import { Shell } from "./Shell";
import { ProjectsPage } from "../features/projects";
import { WizardPage } from "../features/wizard";
import { Workspace } from "../features/workspace";
import { SettingsPage } from "../features/settings";

export const AppRouter: React.FC = () => {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<Shell />}>
          <Route index element={<ProjectsPage />} />
          <Route path="wizard" element={<WizardPage />} />
          <Route path="workspace" element={<Workspace />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
};
