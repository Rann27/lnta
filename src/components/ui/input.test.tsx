import { describe, it, expect, vi, afterEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import "../../lib/i18n";
import { TextField } from "./TextField";
import { SecretField } from "./SecretField";
import { NumberField } from "./NumberField";
import { PathField } from "./PathField";
import { Select } from "./Select";
import { Checkbox } from "./Checkbox";
import { Switch } from "./Switch";
import { SegmentedControl } from "./SegmentedControl";
import { SearchField } from "./SearchField";

afterEach(() => {
  cleanup();
});

describe("TextField component", () => {
  it("associates label with input and supports error/hint ARIA descriptors", () => {
    render(
      <TextField
        label="Nama proyek"
        hint="Nama volume LN"
        error="Nama proyek wajib diisi."
      />,
    );

    const input = screen.getByLabelText("Nama proyek");
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute("aria-invalid", "true");

    const describedBy = input.getAttribute("aria-describedby");
    expect(describedBy).toBeTruthy();

    const errorAlert = screen.getByRole("alert");
    expect(errorAlert.textContent).toContain("Nama proyek wajib diisi.");
    expect(describedBy).toContain(errorAlert.id);
  });
});

describe("SecretField component", () => {
  it("strictly never renders a stored key in the DOM when isStored is true", () => {
    const sensitiveKey = "sk-super-secret-production-key-12345";
    const { container } = render(
      <SecretField
        label="API Key Anthropic"
        isStored={true}
        value={sensitiveKey}
      />,
    );

    // Stored key must not exist anywhere in DOM (html, attributes, text)
    expect(container.innerHTML).not.toContain(sensitiveKey);
    expect(screen.queryByDisplayValue(sensitiveKey)).toBeNull();

    // Must display stored status badge and change button
    expect(screen.getByText("Tersimpan di Credential Manager")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ganti kunci" })).toBeInTheDocument();
  });
});

describe("NumberField component", () => {
  it("shows locale grouping while storing numeric value", () => {
    const handleChange = vi.fn();
    render(
      <NumberField
        label="Batas token"
        value={12480}
        onChange={handleChange}
        unit="token"
      />,
    );

    const input = screen.getByLabelText("Batas token") as HTMLInputElement;
    // In ID locale (default), 12480 is formatted as 12.480
    expect(input.value).toBe("12.480");
    expect(screen.getByText("token")).toBeInTheDocument();
  });
});

describe("SegmentedControl component", () => {
  it("supports keyboard arrow-key navigation and updates selection", () => {
    const handleChange = vi.fn();
    const options = [
      { value: "system", label: "Sistem" },
      { value: "light", label: "Terang" },
      { value: "dark", label: "Gelap" },
    ];

    render(
      <SegmentedControl
        options={options}
        value="system"
        onChange={handleChange}
      />,
    );

    const radios = screen.getAllByRole("radio");
    expect(radios).toHaveLength(3);
    expect(radios[0]).toHaveAttribute("aria-checked", "true");
    expect(radios[0]).toHaveAttribute("tabindex", "0");
    expect(radios[1]).toHaveAttribute("tabindex", "-1");

    // Press ArrowRight to move to next segment
    fireEvent.keyDown(radios[0], { key: "ArrowRight" });
    expect(handleChange).toHaveBeenCalledWith("light");

    // Press ArrowLeft to wrap to last segment
    fireEvent.keyDown(radios[0], { key: "ArrowLeft" });
    expect(handleChange).toHaveBeenCalledWith("dark");
  });
});

describe("Checkbox component", () => {
  it("renders with associated label and handles change", () => {
    const handleChange = vi.fn();
    render(
      <Checkbox
        label="Kirim konteks bab sebelumnya"
        checked={false}
        onChange={handleChange}
      />,
    );

    const checkbox = screen.getByLabelText("Kirim konteks bab sebelumnya");
    expect(checkbox).not.toBeChecked();

    fireEvent.click(checkbox);
    expect(handleChange).toHaveBeenCalled();
  });
});

describe("Switch component", () => {
  it("renders role switch with aria-checked", () => {
    const handleChange = vi.fn();
    render(
      <Switch
        label="Notifikasi suara"
        checked={true}
        onChange={handleChange}
      />,
    );

    const switchEl = screen.getByRole("switch", { name: "Notifikasi suara" });
    expect(switchEl).toBeChecked();
    expect(switchEl).toHaveAttribute("aria-checked", "true");
  });
});

describe("Select component", () => {
  it("renders select with associated label and error state", () => {
    render(
      <Select
        label="Bahasa UI"
        options={[
          { value: "id", label: "Bahasa Indonesia" },
          { value: "en", label: "English" },
        ]}
        error="Pilihan bahasa tidak valid."
      />,
    );

    const select = screen.getByLabelText("Bahasa UI");
    expect(select).toBeInTheDocument();
    expect(select).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByRole("alert").textContent).toContain("Pilihan bahasa tidak valid.");
  });
});

describe("PathField component", () => {
  it("renders path input with browse button", () => {
    const handleBrowse = vi.fn();
    render(
      <PathField
        label="Folder proyek"
        value="D:/LN/Volume1"
        onBrowse={handleBrowse}
      />,
    );

    expect(screen.getByLabelText("Folder proyek")).toHaveValue("D:/LN/Volume1");
    const browseBtn = screen.getByRole("button", { name: "Pilih" });
    fireEvent.click(browseBtn);
    expect(handleBrowse).toHaveBeenCalled();
  });
});

describe("SearchField component", () => {
  it("renders search input and clears value with clear button", () => {
    const handleChange = vi.fn();
    render(
      <SearchField
        value="karamawari"
        onChange={handleChange}
        resultCountText="3 hasil ditemukan"
      />,
    );

    expect(screen.getByDisplayValue("karamawari")).toBeInTheDocument();
    expect(screen.getByText("3 hasil ditemukan")).toBeInTheDocument();

    const clearBtn = screen.getByRole("button", { name: "Hapus pencarian" });
    fireEvent.click(clearBtn);
    expect(handleChange).toHaveBeenCalledWith("");
  });
});
