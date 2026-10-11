import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import "../../lib/i18n";
import { MemoryRouter } from "react-router-dom";
import { ProjectTab } from "./ProjectTab";
import { useWorkspaceStore } from "../workspace/store";
import { setIpcTransport } from "../../lib/ipc";
import { PreviewTransport } from "../../lib/preview/transport";

const PATH = "D:/LN/Synthetic-Volume-1";

function renderTab() {
  return render(
    <MemoryRouter>
      <ProjectTab />
    </MemoryRouter>
  );
}

describe("ProjectTab", () => {
  let transport: PreviewTransport;

  beforeEach(async () => {
    transport = new PreviewTransport("project-idle-blocker");
    setIpcTransport(transport);
    useWorkspaceStore.setState({
      projectPath: PATH,
      chapters: [],
      imagePageIds: {},
      glossaryCount: 3,
      preflight: await transport.runPreflight(PATH),
      runState: { state: "idle", active: false, current_chapter_id: null, latest_run: null },
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("shows the saved settings and saves only after a change", async () => {
    const save = vi.spyOn(transport, "saveProjectConfig");
    renderTab();

    const saveButton = await screen.findByRole("button", { name: /^(Simpan|Save)$/ });
    expect(saveButton).toBeDisabled();
    expect(screen.getByText(/glossary\.md, 3 (entri|entries)/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("switch", { name: /Kirim seluruh glosarium|Send full glossary/i }));
    expect(saveButton).not.toBeDisabled();
    fireEvent.click(saveButton);

    await waitFor(() => expect(save).toHaveBeenCalledTimes(1));
    expect(save.mock.calls[0][0]).toBe(PATH);
  });

  it("locks settings while a run is active", async () => {
    useWorkspaceStore.setState({
      runState: { state: "running", active: true, current_chapter_id: null, latest_run: null },
    });
    renderTab();

    expect(await screen.findByText(/dikunci selama run|locked while a run/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Reset proyek|Reset project/i })).toBeDisabled();
  });

  it("asks before resetting the project", async () => {
    const reset = vi.spyOn(transport, "resetProject");
    renderTab();

    fireEvent.click(await screen.findByRole("button", { name: /Reset proyek|Reset project/i }));
    expect(reset).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /Hapus data \.lnta|Delete \.lnta data/i }));

    await waitFor(() => expect(reset).toHaveBeenCalledWith(PATH));
  });
});
