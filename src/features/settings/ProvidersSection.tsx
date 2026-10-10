import React, { useState } from "react";
import {
  PlugsConnected,
  Plugs,
  Plus,
  Trash,
  ArrowClockwise,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { Dialog } from "../../components/ui/Dialog";
import { SecretField } from "../../components/ui/SecretField";
import { TextField } from "../../components/ui/TextField";
import { Select } from "../../components/ui/Select";
import { Switch } from "../../components/ui/Switch";
import { useSettingsStore } from "./store";
import type { ProviderConfigEntry } from "../../lib/bindings";
import styles from "./ProvidersSection.module.css";

const DEFAULT_URLS: Record<string, string> = {
  openai_compat: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  gemini: "https://generativelanguage.googleapis.com",
  cli_claude: "local://claude",
  cli_gemini: "local://gemini",
  cli_codex: "local://codex",
};

export const ProvidersSection: React.FC = () => {
  const { t } = useTranslation();
  const {
    providers,
    apiKeysStatus,
    testResults,
    isTesting,
    saveProvider,
    deleteProvider,
    setApiKey,
    clearApiKey,
    testProvider,
    fetchProviderModels,
  } = useSettingsStore();

  const [isAddOpen, setIsAddOpen] = useState(false);
  const [newId, setNewId] = useState("");
  const [newName, setNewName] = useState("");
  const [newType, setNewType] = useState<
    "openai_compat" | "anthropic" | "gemini" | "cli_claude" | "cli_gemini" | "cli_codex"
  >("openai_compat");
  const [newBaseUrl, setNewBaseUrl] = useState(DEFAULT_URLS.openai_compat);
  const [newKey, setNewKey] = useState("");
  const [isFetchingModels, setIsFetchingModels] = useState<Record<string, boolean>>({});

  const handleTypeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value as
      | "openai_compat"
      | "anthropic"
      | "gemini"
      | "cli_claude"
      | "cli_gemini"
      | "cli_codex";
    setNewType(val);
    if (!newBaseUrl || Object.values(DEFAULT_URLS).includes(newBaseUrl)) {
      setNewBaseUrl(DEFAULT_URLS[val] ?? "");
    }
  };

  const handleOpenAdd = () => {
    setNewId("");
    setNewName("");
    setNewType("openai_compat");
    setNewBaseUrl(DEFAULT_URLS.openai_compat);
    setNewKey("");
    setIsAddOpen(true);
  };

  const handleSaveNewProvider = async () => {
    const generatedId =
      newId.trim() ||
      `${newType}_${Date.now().toString(36)}`;
    const entry: ProviderConfigEntry = {
      id: generatedId,
      name: newName.trim() || generatedId,
      provider_type: newType,
      base_url: newBaseUrl.trim(),
      active: true,
    };

    await saveProvider(entry);
    if (newKey.trim()) {
      await setApiKey(generatedId, newKey.trim());
    }
    setIsAddOpen(false);
  };

  const handleToggleActive = async (provider: ProviderConfigEntry) => {
    await saveProvider({
      ...provider,
      active: !provider.active,
    });
  };

  const handleFetchModels = async (provider: ProviderConfigEntry) => {
    setIsFetchingModels((prev) => ({ ...prev, [provider.id]: true }));
    try {
      await fetchProviderModels(provider.provider_type, provider.base_url);
    } catch {
      // ignore in UI, store updates
    } finally {
      setIsFetchingModels((prev) => ({ ...prev, [provider.id]: false }));
    }
  };

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <div className={styles.titleGroup}>
          <h2 className={styles.title}>{t("settings.providers.title")}</h2>
          <p className={styles.subtitle}>{t("settings.providers.subtitle")}</p>
        </div>

        <Button
          variant="primary"
          icon={<Plus size={16} />}
          onClick={handleOpenAdd}
        >
          {t("settings.providers.add_button")}
        </Button>
      </header>

      {providers.length === 0 ? (
        <div className={styles.emptyState}>
          <p>{t("settings.providers.empty_text")}</p>
          <Button variant="secondary" onClick={handleOpenAdd}>
            {t("settings.providers.add_button")}
          </Button>
        </div>
      ) : (
        <div className={styles.providerList}>
          {providers.map((p) => {
            const hasKey = Boolean(apiKeysStatus[p.id]);
            const testResult = testResults[p.id];
            const testing = Boolean(isTesting[p.id]);
            const fetching = Boolean(isFetchingModels[p.id]);

            return (
              <div key={p.id} className={styles.providerCard}>
                <div className={styles.cardHeader}>
                  <div className={styles.cardTitleWrap}>
                    <span className={styles.providerName}>{p.name}</span>
                    <span className={styles.providerTypeTag}>{p.provider_type}</span>
                  </div>

                  <div className={styles.cardActions}>
                    <Switch
                      label={t("settings.providers.active_toggle")}
                      checked={p.active}
                      onChange={() => handleToggleActive(p)}
                    />
                    <Button
                      size="sm"
                      variant="ghost"
                      icon={<Trash size={16} />}
                      onClick={() => deleteProvider(p.id)}
                      aria-label={t("common.delete")}
                    />
                  </div>
                </div>

                <div className={styles.cardBody}>
                  <div className={styles.baseUrlWrap}>
                    <span className={styles.baseUrlLabel}>
                      {t("settings.providers.base_url")}:
                    </span>
                    <span className={styles.baseUrlValue}>{p.base_url}</span>
                  </div>

                  <SecretField
                    label={t("settings.providers.api_key_label")}
                    isStored={hasKey}
                    onTestConnection={() => testProvider(p.id)}
                    isTesting={testing}
                    onChange={(key) => setApiKey(p.id, key)}
                    onClear={() => clearApiKey(p.id)}
                  />

                  {testResult && (
                    testResult.success ? (
                      <div className={styles.testResultSuccess}>
                        <PlugsConnected size={16} />
                        <span>
                          {t("settings.providers.test_success", {
                            latency: testResult.latency_ms.toString(),
                          })}
                        </span>
                      </div>
                    ) : (
                      <div className={styles.testResultError}>
                        <Plugs size={16} />
                        <span>
                          {t("settings.providers.test_error", {
                            error: testResult.error ?? "Failed",
                          })}
                        </span>
                      </div>
                    )
                  )}

                  <div>
                    <Button
                      size="sm"
                      variant="secondary"
                      icon={<ArrowClockwise size={14} />}
                      disabled={fetching}
                      onClick={() => handleFetchModels(p)}
                    >
                      {fetching
                        ? t("settings.providers.fetching_models")
                        : t("settings.providers.fetch_models_button")}
                    </Button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Add Provider Dialog */}
      <Dialog
        isOpen={isAddOpen}
        onClose={() => setIsAddOpen(false)}
        title={t("settings.providers.dialog_title")}
        actions={
          <>
            <Button variant="secondary" onClick={() => setIsAddOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="primary" onClick={handleSaveNewProvider}>
              {t("common.save")}
            </Button>
          </>
        }
      >
        <div className={styles.dialogForm}>
          <Select
            label={t("settings.providers.provider_type_label")}
            value={newType}
            onChange={handleTypeChange}
            options={[
              { value: "openai_compat", label: "OpenAI-compatible" },
              { value: "anthropic", label: "Anthropic" },
              { value: "gemini", label: "Google Gemini" },
              { value: "cli_claude", label: "Claude Code CLI (Experimental)" },
              { value: "cli_gemini", label: "Gemini CLI (Experimental)" },
              { value: "cli_codex", label: "Codex CLI (Experimental)" },
            ]}
          />

          <TextField
            label={t("settings.providers.name_label")}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="misal: OpenAI / OpenRouter"
          />

          <TextField
            label={t("settings.providers.id_label")}
            value={newId}
            onChange={(e) => setNewId(e.target.value)}
            placeholder={newType}
            hint={t("settings.providers.id_hint")}
          />

          <TextField
            label={t("settings.providers.base_url_label")}
            value={newBaseUrl}
            onChange={(e) => setNewBaseUrl(e.target.value)}
          />

          <SecretField
            label={t("settings.providers.api_key_label")}
            isStored={false}
            value={newKey}
            onChange={setNewKey}
            placeholder="sk-..."
          />
        </div>
      </Dialog>
    </div>
  );
};
