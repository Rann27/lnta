import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import "../../lib/i18n";
import { Button } from "./Button";
import { ProgressBar } from "./ProgressBar";
import { IconButton } from "./IconButton";
import { StatusTag, type StatusType } from "./StatusTag";
import { FolderOpen } from "@phosphor-icons/react";

describe("Button component", () => {
  it("renders with label and handles disabled attribute", () => {
    render(<Button disabled>Klik saya</Button>);
    const btn = screen.getByRole("button", { name: "Klik saya" });
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("aria-disabled", "true");
  });

  it("renders disabled reason and associates via aria-describedby", () => {
    render(
      <Button disabledReason="File instruction belum ditemukan.">
        Jalankan volume
      </Button>,
    );
    const btn = screen.getByRole("button", { name: "Jalankan volume" });
    expect(btn).toBeDisabled();

    const describedById = btn.getAttribute("aria-describedby");
    expect(describedById).toBeTruthy();

    const hint = screen.getByRole("status");
    expect(hint.id).toBe(describedById);
    expect(hint.textContent).toBe("File instruction belum ditemukan.");
  });
});

describe("ProgressBar component", () => {
  it("renders proper ARIA values and label", () => {
    render(<ProgressBar label="Proses penerjemahan" value={42} max={100} />);
    const bar = screen.getByRole("progressbar", { name: "Proses penerjemahan" });

    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "100");
    expect(bar).toHaveAttribute("aria-valuenow", "42");
    expect(bar).toHaveAttribute("aria-label", "Proses penerjemahan");
  });

  it("clamps values within 0 and max", () => {
    render(<ProgressBar label="Clamped bar" value={150} max={100} />);
    const bar = screen.getByRole("progressbar", { name: "Clamped bar" });
    expect(bar).toHaveAttribute("aria-valuenow", "100");
  });
});

describe("IconButton component", () => {
  it("requires and applies aria-label", () => {
    render(
      <IconButton
        aria-label="Buka folder"
        icon={<FolderOpen size={16} />}
      />,
    );
    expect(screen.getByRole("button", { name: "Buka folder" })).toBeInTheDocument();
  });
});

describe("StatusTag component", () => {
  it("renders all eight statuses", () => {
    const statuses: StatusType[] = [
      "queued",
      "running",
      "paused",
      "done",
      "needs_attention",
      "blocked",
      "frozen",
      "source_changed",
    ];

    const { container } = render(
      <div>
        {statuses.map((st) => (
          <StatusTag key={st} status={st} />
        ))}
      </div>,
    );

    for (const st of statuses) {
      expect(container.querySelector(`[data-status="${st}"]`)).toBeTruthy();
    }
  });
});
