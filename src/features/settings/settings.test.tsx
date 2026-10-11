import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import "../../lib/i18n";
import { useSettingsStore } from "./store";
import { SettingsPage } from "./SettingsPage";
import { AboutDialog } from "./AboutDialog";
import { ModelPicker } from "../../components/ui/ModelPicker";
import type { ModelEntry } from "../../lib/bindings";

const SAMPLE_MODELS: ModelEntry[] = [
  {
    provider: "anthropic",
    model_id: "claude-3-5-sonnet",
    display_name: "Claude 3.5 Sonnet",
    context_window: 200000,
    max_output_tokens: 8192,
    pricing: {
      input_per_million: 3.0,
      cached_input_per_million: 0.3,
      output_per_million: 15.0,
    },
    value_source: "default_registry",
    supports_reasoning: false,
  },
  {
    provider: "openai",
    model_id: "gpt-4o",
    display_name: "GPT-4o",
    context_window: 128000,
    max_output_tokens: 4096,
    pricing: {
      input_per_million: 2.5,
      cached_input_per_million: 1.25,
      output_per_million: 10.0,
    },
    value_source: "default_registry",
    supports_reasoning: false,
  },
  {
    provider: "custom",
    model_id: "model-without-limit",
    display_name: "Model Without Limit",
    context_window: 32000,
    max_output_tokens: 0,
    pricing: {
      input_per_million: 1.0,
      cached_input_per_million: 0,
      output_per_million: 2.0,
    },
    value_source: "user_supplied",
    supports_reasoning: true,
  },
];

describe("Settings Store", () => {
  beforeEach(() => {
    useSettingsStore.setState({
      providers: [
        {
          id: "openai",
          name: "OpenAI",
          provider_type: "openai_compat",
          base_url: "https://api.openai.com/v1",
          active: true,
        },
      ],
      apiKeysStatus: { openai: true },
      testResults: {},
      isTesting: {},
      models: SAMPLE_MODELS,
      appInfo: {
        version: "0.1.0",
        license: "MIT + Commons Clause",
        repo_url: "https://github.com/Rann27/lnta",
      },
      globalSettings: {
        theme: "dark",
        ui_language: "id",
        notifications_enabled: true,
        roles: {
          translator: {
            primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
            fallbacks: [{ provider: "openai", model_id: "gpt-4o" }],
            temperature: 0.3,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
          repair: {
            primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
            fallbacks: [],
            temperature: 0.2,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
          qa_reviewer: {
            primary: { provider: "openai", model_id: "gpt-4o" },
            fallbacks: [],
            temperature: 0.1,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
          qa_fixer: {
            primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
            fallbacks: [],
            temperature: 0.2,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
        },
        providers: [],
        custom_models: [],
        recent_projects: [],
      },
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("updates and stores API key in keyring status without exposing key", async () => {
    const store = useSettingsStore.getState();
    await store.setApiKey("openai", "sk-secret-new-key");

    expect(useSettingsStore.getState().apiKeysStatus.openai).toBe(true);
    // Plaintext key is never added to the store state
    expect(JSON.stringify(useSettingsStore.getState())).not.toContain("sk-secret-new-key");
  });

  it("clears API key correctly", async () => {
    const store = useSettingsStore.getState();
    await store.clearApiKey("openai");
    expect(useSettingsStore.getState().apiKeysStatus.openai).toBe(false);
  });

  it("manages role fallbacks (add, reorder, remove)", async () => {
    const store = useSettingsStore.getState();
    await store.addRoleFallback("translator", {
      provider: "custom",
      model_id: "model-without-limit",
    });

    let fallbacks = useSettingsStore.getState().globalSettings!.roles.translator.fallbacks;
    expect(fallbacks).toHaveLength(2);
    expect(fallbacks[1].model_id).toBe("model-without-limit");

    // Reorder
    await store.reorderRoleFallback("translator", 1, 0);
    fallbacks = useSettingsStore.getState().globalSettings!.roles.translator.fallbacks;
    expect(fallbacks[0].model_id).toBe("model-without-limit");

    // Remove
    await store.removeRoleFallback("translator", 0);
    fallbacks = useSettingsStore.getState().globalSettings!.roles.translator.fallbacks;
    expect(fallbacks).toHaveLength(1);
    expect(fallbacks[0].model_id).toBe("gpt-4o");
  });

  it("tests provider connection and records latency", async () => {
    const store = useSettingsStore.getState();
    const result = await store.testProvider("openai");
    expect(result.success).toBe(true);
    expect(useSettingsStore.getState().testResults.openai).toEqual(result);
  });
});

describe("AboutDialog", () => {
  afterEach(() => {
    cleanup();
  });

  it("displays app attribution, version, license, and repository URL", () => {
    useSettingsStore.setState({
      appInfo: {
        version: "0.1.0",
        license: "MIT + Commons Clause",
        repo_url: "https://github.com/Rann27/lnta",
      },
    });

    const onClose = vi.fn();
    render(<AboutDialog isOpen={true} onClose={onClose} />);

    expect(screen.getByText("LNTA")).toBeInTheDocument();
    expect(screen.getByText(/LNTA oleh Rann27/i)).toBeInTheDocument();
    expect(screen.getByText("v0.1.0")).toBeInTheDocument();
    expect(screen.getByText("MIT + Commons Clause")).toBeInTheDocument();
    expect(screen.getByText("https://github.com/Rann27/lnta")).toBeInTheDocument();

    fireEvent.click(screen.getByText(/Tutup/i));
    expect(onClose).toHaveBeenCalled();
  });
});

describe("ModelPicker", () => {
  afterEach(() => {
    cleanup();
  });

  it("displays trigger with selected model and opens listbox with warning for missing output", async () => {
    const onChange = vi.fn();
    render(
      <MemoryRouter>
        <ModelPicker
          value={{ provider: "anthropic", model_id: "claude-3-5-sonnet" }}
          models={SAMPLE_MODELS}
          onChange={onChange}
        />
      </MemoryRouter>
    );

    const trigger = screen.getByRole("combobox");
    expect(trigger).toHaveTextContent("Claude 3.5 Sonnet");

    fireEvent.click(trigger);

    // Listbox should be visible
    expect(screen.getByRole("listbox")).toBeInTheDocument();

    // Check warning for missing output limit
    expect(screen.getByText(/batas output belum diisi/i)).toBeInTheDocument();

    // Select GPT-4o
    fireEvent.click(screen.getByText("GPT-4o"));
    expect(onChange).toHaveBeenCalledWith({
      provider: "openai",
      model_id: "gpt-4o",
    });
  });

  it("filters models by typing search query", () => {
    render(
      <MemoryRouter>
        <ModelPicker models={SAMPLE_MODELS} />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("combobox"));

    const searchInput = screen.getByRole("textbox");
    fireEvent.change(searchInput, { target: { value: "claude" } });

    expect(screen.getByText("Claude 3.5 Sonnet")).toBeInTheDocument();
    expect(screen.queryByText("GPT-4o")).not.toBeInTheDocument();
  });
});

describe("SettingsPage navigation", () => {
  beforeEach(() => {
    useSettingsStore.setState({
      providers: [
        {
          id: "openai",
          name: "OpenAI",
          provider_type: "openai_compat",
          base_url: "https://api.openai.com/v1",
          active: true,
        },
      ],
      apiKeysStatus: { openai: true },
      testResults: {},
      isTesting: {},
      models: SAMPLE_MODELS,
      appInfo: {
        version: "0.1.0",
        license: "MIT + Commons Clause",
        repo_url: "https://github.com/Rann27/lnta",
      },
      globalSettings: {
        theme: "dark",
        ui_language: "id",
        notifications_enabled: true,
        roles: {
          translator: {
            primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
            fallbacks: [{ provider: "openai", model_id: "gpt-4o" }],
            temperature: 0.3,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
          repair: {
            primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
            fallbacks: [],
            temperature: 0.2,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
          qa_reviewer: {
            primary: { provider: "openai", model_id: "gpt-4o" },
            fallbacks: [],
            temperature: 0.1,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
          qa_fixer: {
            primary: { provider: "anthropic", model_id: "claude-3-5-sonnet" },
            fallbacks: [],
            temperature: 0.2,
            max_output_tokens: 4096,
            reasoning_effort: null,
          },
        },
        providers: [],
        custom_models: [],
        recent_projects: [],
      },
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders Settings page and switches tabs", async () => {
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>
    );

    expect(screen.getByRole("heading", { name: "Pengaturan" })).toBeInTheDocument();

    // Default tab is Provider & Kunci - wait for loadSettings to complete
    await waitFor(() => {
      expect(
        screen.getByText(/Kunci API disimpan di keyring sistem/i)
      ).toBeInTheDocument();
    });

    // Switch to Model tab
    fireEvent.click(screen.getByRole("tab", { name: "Model" }));
    await waitFor(() => {
      expect(
        screen.getByText(/Daftar model yang tersedia untuk alur/i)
      ).toBeInTheDocument();
    });

    // Switch to Roles tab
    fireEvent.click(screen.getByRole("tab", { name: "Peran & Fallback" }));
    await waitFor(() => {
      expect(
        screen.getByText(/Tentukan model utama dan urutan fallback/i)
      ).toBeInTheDocument();
    });
  });

  it("opens About dialog when clicking button", () => {
    render(
      <MemoryRouter>
        <SettingsPage />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByText("Tentang LNTA"));
    expect(screen.getByText(/LNTA oleh Rann27/i)).toBeInTheDocument();
    expect(screen.getByText("MIT + Commons Clause")).toBeInTheDocument();
  });
});

describe("RolesSection parameters", () => {
  afterEach(() => cleanup());

  it("offers effort levels and hides temperature for models that set their own", async () => {
    const { RolesSection } = await import("./RolesSection");
    const { PREVIEW_MODELS } = await import("../../lib/preview/scenarios");
    const role = {
      primary: { provider: "anthropic", model_id: "claude-sonnet-5-5" },
      fallbacks: [],
      temperature: null,
      max_output_tokens: null,
      reasoning_effort: null,
    };
    const gemini = { ...role, primary: { provider: "gemini", model_id: "gemini-3.8-flash" } };
    useSettingsStore.setState({
      models: PREVIEW_MODELS,
      globalSettings: {
        theme: "system",
        ui_language: "id",
        notifications_enabled: true,
        roles: { translator: role, repair: gemini, qa_reviewer: role, qa_fixer: role },
        providers: [],
        custom_models: [],
        recent_projects: [],
      },
    });

    render(<RolesSection />);

    expect(screen.getAllByText("Model ini mengatur temperature sendiri.")).toHaveLength(3);
    expect(screen.getByText("Disarankan provider: 1")).toBeInTheDocument();
    const effortSelects = screen.getAllByRole("combobox", { name: "Reasoning effort" });
    expect(effortSelects).toHaveLength(4);
    expect(effortSelects[0]).toHaveTextContent("Bawaan model (high)");
  });
});
