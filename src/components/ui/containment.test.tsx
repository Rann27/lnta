import { describe, it, expect, vi, afterEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import "../../lib/i18n";
import { Panel } from "./Panel";
import { SplitDivider } from "./SplitDivider";
import { Banner } from "./Banner";
import { Dialog } from "./Dialog";
import { Popover } from "./Popover";
import { ContextMenu } from "./ContextMenu";
import { Toast } from "./Toast";
import { Disclosure } from "./Disclosure";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("Banner component", () => {
  it("uses role='alert' for blocker variant and role='status' for attention variant", () => {
    const { rerender } = render(
      <Banner
        variant="blocker"
        title="Translations.docx terkunci"
        description="Tutup file di Word sebelum melanjutkan."
      />,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("Translations.docx terkunci")).toBeInTheDocument();

    rerender(
      <Banner
        variant="attention"
        title="Glosarium memiliki 3 konflik"
      />,
    );

    expect(screen.getByRole("status")).toBeInTheDocument();
  });
});

describe("SplitDivider component", () => {
  it("is resizable by keyboard arrow keys and has role='separator'", () => {
    const handleChange = vi.fn();
    render(
      <SplitDivider
        value={300}
        min={240}
        max={420}
        step={16}
        onChange={handleChange}
      />,
    );

    const divider = screen.getByRole("separator");
    expect(divider).toHaveAttribute("aria-valuenow", "300");
    expect(divider).toHaveAttribute("aria-orientation", "vertical");

    fireEvent.keyDown(divider, { key: "ArrowRight" });
    expect(handleChange).toHaveBeenCalledWith(316);

    fireEvent.keyDown(divider, { key: "ArrowLeft" });
    expect(handleChange).toHaveBeenCalledWith(284);
  });
});

describe("Dialog component", () => {
  it("traps focus, closes on Esc, and returns focus to trigger element", () => {
    const handleClose = vi.fn();
    const { rerender } = render(
      <div>
        <button id="trigger">Buka modal</button>
        <Dialog
          isOpen={true}
          onClose={handleClose}
          title="Konfirmasi aksi"
          actions={
            <>
              <button id="btn-cancel">Batal</button>
              <button id="btn-ok">Lanjutkan</button>
            </>
          }
        />
      </div>,
    );

    const dialog = screen.getByRole("dialog");
    expect(dialog).toBeInTheDocument();

    // Escape closes dialog
    fireEvent.keyDown(window, { key: "Escape" });
    expect(handleClose).toHaveBeenCalled();

    // Closing returns focus
    rerender(
      <div>
        <button id="trigger">Buka modal</button>
        <Dialog isOpen={false} onClose={handleClose} title="Konfirmasi aksi" />
      </div>,
    );
  });

  it("destructive variant focuses Cancel first and ignores scrim clicks", () => {
    const handleClose = vi.fn();
    render(
      <Dialog
        isOpen={true}
        onClose={handleClose}
        variant="destructive"
        title="Hapus semua progres?"
        actions={
          <>
            <button id="cancel-btn">Batal</button>
            <button id="delete-btn">Hapus</button>
          </>
        }
      />,
    );

    const cancelBtn = screen.getByRole("button", { name: "Batal" });
    expect(document.activeElement).toBe(cancelBtn);

    const scrim = screen.getByTestId("dialog-scrim");
    fireEvent.click(scrim);
    // Destructive dialog strictly ignores scrim clicks
    expect(handleClose).not.toHaveBeenCalled();
  });
});

describe("Toast component", () => {
  it("auto-hides after 6 s and pauses on hover/focus", () => {
    vi.useFakeTimers();
    const handleClose = vi.fn();

    render(
      <Toast
        isOpen={true}
        onClose={handleClose}
        message="Glosarium berhasil disimpan."
        durationMs={6000}
      />,
    );

    const toast = screen.getByTestId("toast-container");
    expect(toast).toBeInTheDocument();

    // Advance 3s
    vi.advanceTimersByTime(3000);
    expect(handleClose).not.toHaveBeenCalled();

    // Mouse enter pauses the timer
    fireEvent.mouseEnter(toast);
    vi.advanceTimersByTime(5000);
    expect(handleClose).not.toHaveBeenCalled();

    // Mouse leave resumes the timer for the remaining 3s
    fireEvent.mouseLeave(toast);
    vi.advanceTimersByTime(2900);
    expect(handleClose).not.toHaveBeenCalled();

    vi.advanceTimersByTime(200);
    expect(handleClose).toHaveBeenCalledTimes(1);
  });
});

describe("Disclosure component", () => {
  it("expands and collapses with aria-expanded", () => {
    render(
      <Disclosure title="Detail teknis">
        <p>Model: claude-3-7-sonnet</p>
      </Disclosure>,
    );

    const button = screen.getByRole("button", { name: "Detail teknis" });
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("Model: claude-3-7-sonnet")).toBeNull();

    fireEvent.click(button);
    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Model: claude-3-7-sonnet")).toBeInTheDocument();
  });
});

describe("Popover component", () => {
  it("toggles popover and closes on Escape", () => {
    render(
      <Popover
        trigger={<button>Pemicu</button>}
        content={<div>Konten popover</div>}
      />,
    );

    const trigger = screen.getByRole("button", { name: "Pemicu" });
    fireEvent.click(trigger);
    expect(screen.getByText("Konten popover")).toBeInTheDocument();

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByText("Konten popover")).toBeNull();
  });
});

describe("ContextMenu component", () => {
  it("opens menu on contextmenu event and responds to items", () => {
    const handleAction = vi.fn();
    render(
      <ContextMenu
        items={[
          { label: "Buka file", onClick: handleAction },
        ]}
      >
        <div data-testid="target">Klik kanan saya</div>
      </ContextMenu>,
    );

    const target = screen.getByTestId("target");
    fireEvent.contextMenu(target, { clientX: 100, clientY: 200 });

    const menuItem = screen.getByRole("menuitem", { name: "Buka file" });
    expect(menuItem).toBeInTheDocument();

    fireEvent.click(menuItem);
    expect(handleAction).toHaveBeenCalled();
  });
});

describe("Panel component", () => {
  it("renders with panel styling", () => {
    const { container } = render(<Panel>Isi panel</Panel>);
    expect(container.firstChild).toHaveTextContent("Isi panel");
  });
});
