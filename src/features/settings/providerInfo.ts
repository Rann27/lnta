/** Provider types the add dialog offers, with their default base URL. */
export const PROVIDER_TYPES = [
  "openai_compat",
  "anthropic",
  "gemini",
  "cli_claude",
  "cli_codex",
  "cli_antigravity",
] as const;

export type ProviderType = (typeof PROVIDER_TYPES)[number];

export const DEFAULT_URLS: Record<ProviderType, string> = {
  openai_compat: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  gemini: "https://generativelanguage.googleapis.com",
  cli_claude: "local://claude",
  cli_codex: "local://codex",
  cli_antigravity: "local://agy",
};

export function isCliType(providerType: string): boolean {
  return providerType.startsWith("cli_");
}

/**
 * Where to get an API key for each built-in provider. Console home pages are used where a
 * deep link to the key page could move.
 */
export const KEY_PAGES: Record<string, string> = {
  anthropic: "https://platform.claude.com/",
  openai: "https://platform.openai.com/api-keys",
  gemini: "https://aistudio.google.com/apikey",
  xai: "https://console.x.ai/",
  deepseek: "https://platform.deepseek.com/api_keys",
  zai: "https://z.ai/model-api",
  qwen: "https://modelstudio.console.alibabacloud.com/",
  minimax: "https://platform.minimax.io/",
  xiaomi: "https://platform.xiaomimimo.com/",
  moonshot: "https://platform.moonshot.ai/console/api-keys",
  mistral: "https://console.mistral.ai/api-keys",
  openrouter: "https://openrouter.ai/settings/keys",
};

/** Install guides for the subscription CLIs, by provider type. */
export const CLI_INSTALL_PAGES: Record<string, string> = {
  cli_claude: "https://code.claude.com/docs/en/setup",
  cli_codex: "https://github.com/openai/codex",
  cli_antigravity: "https://antigravity.google/docs/cli",
};
