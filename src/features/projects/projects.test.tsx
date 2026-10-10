import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import "../../lib/i18n";
import { render, screen, cleanup } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { useProjectsStore } from "./store";
import { ProjectsPage } from "./ProjectsPage";
import { setIpcTransport } from "../../lib/ipc";
import { PreviewTransport } from "../../lib/preview/transport";

describe("Projects feature and Beranda", () => {
  beforeEach(() => {
    useProjectsStore.setState({
      recentProjects: [],
      isLoading: false,
      error: null,
      activeProject: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("loads recent projects into store", async () => {
    setIpcTransport(new PreviewTransport("project-idle-blocker"));
    await useProjectsStore.getState().loadRecentProjects();

    const state = useProjectsStore.getState();
    expect(state.recentProjects.length).toBeGreaterThan(0);
    expect(state.recentProjects[0].name).toBe("Synthetic-Volume-1");
  });

  it("opens project and stores activeProject", async () => {
    setIpcTransport(new PreviewTransport("project-idle-blocker"));
    const res = await useProjectsStore.getState().openProject("D:/test");
    expect(res).toBeDefined();
    expect(useProjectsStore.getState().activeProject).toBe(res);
  });

  it("renders empty state when there are no recent projects", async () => {
    setIpcTransport(new PreviewTransport("no-projects"));

    render(
      <MemoryRouter>
        <ProjectsPage />
      </MemoryRouter>
    );

    // After mount, store loads empty list
    await vi.waitFor(() => {
      expect(screen.getByText(/Belum ada proyek|No projects yet/i)).toBeInTheDocument();
    });
  });

  it("renders recent project cards when projects exist", async () => {
    setIpcTransport(new PreviewTransport("project-idle-blocker"));

    render(
      <MemoryRouter>
        <ProjectsPage />
      </MemoryRouter>
    );

    await vi.waitFor(() => {
      expect(screen.getByText("Synthetic-Volume-1")).toBeInTheDocument();
    });
  });
});
