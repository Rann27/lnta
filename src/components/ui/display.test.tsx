import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import "../../lib/i18n";
import { Button } from "./Button";
import { ProgressBar } from "./ProgressBar";
import { IconButton } from "./IconButton";
import { StatusTag, type StatusType } from "./StatusTag";
import { SegmentRuler } from "./SegmentRuler";
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

describe("SegmentRuler component", () => {
  it("renders passive ruler with cells and aria-hidden", () => {
    const segments = [
      { idx: 1, status: "done" as const },
      { idx: 2, status: "live" as const },
      { idx: 3, status: "pending" as const },
    ];
    const { container } = render(<SegmentRuler segments={segments} variant="rail" />);
    const ruler = container.firstElementChild as HTMLElement;
    expect(ruler).toHaveAttribute("aria-hidden", "true");
    expect(container.querySelectorAll('[data-status="done"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-status="live"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-status="pending"]')).toHaveLength(1);
  });

  it("performs binning with worst status priority (fail > flag > live > done)", () => {
    const segments = [
      { idx: 1, status: "done" as const },
      { idx: 2, status: "flag" as const },
      { idx: 3, status: "done" as const },
      { idx: 4, status: "fail" as const },
      { idx: 5, status: "done" as const },
      { idx: 6, status: "live" as const },
    ];
    // Bin into 2 bins: [1,2,3] -> worst is "flag", [4,5,6] -> worst is "fail"
    const { container } = render(<SegmentRuler segments={segments} maxBins={2} />);
    const cells = container.querySelectorAll("[data-status]");
    expect(cells).toHaveLength(2);
    expect(cells[0]).toHaveAttribute("data-status", "flag");
    expect(cells[1]).toHaveAttribute("data-status", "fail");
  });

  it("renders interactive cells as buttons and responds to click", () => {
    let clickedIndex = -1;
    const segments = [
      { idx: 10, status: "done" as const },
      { idx: 11, status: "live" as const },
    ];
    const { container } = render(
      <SegmentRuler
        segments={segments}
        variant="header"
        interactive
        onSelectSegment={(idx) => {
          clickedIndex = idx;
        }}
      />
    );
    const buttons = container.querySelectorAll("button");
    expect(buttons).toHaveLength(2);
    buttons[1].click();
    expect(clickedIndex).toBe(11);
  });
});

