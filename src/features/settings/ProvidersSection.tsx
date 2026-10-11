import React, { useState } from "react";
import {
  PlugsConnected,
  Plugs,
  Plus,
  Trash,
  ArrowClockwise,
  ArrowSquareOut,
  Terminal,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { Dialog } from "../../components/ui/Dialog";
import { SecretField } from "../../components/ui/SecretField";
import { TextField } from "../../components/ui/TextField";
import { Select } from "../../components/ui/Select";
import { Switch } from "../../components/ui/Switch";
import { useSettingsStore } from "./store";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errors";
import type { ProviderConfigEntry } from "../../lib/bindings";
import {
  CLI_INSTALL_PAGES,
  DEFAULT_URLS,
  KEY_PAGES,
  PROVIDER_TYPES,
  isCliType,
  type ProviderType,
} from "./providerInfo";
import styles from "./ProvidersSection.module.css";

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
  const [newType, setNewType] = useState<ProviderType>("openai_compat");
  const [newBaseUrl, setNewBaseUrl] = useState(DEFAULT_URLS.openai_compat);
  const [newKey, setNewKey] = useState("");
  const [isFetchingModels, setIsFetchingModels] = useState<Record<string, boolean>>({});
  const [fetchNotes, setFetchNotes] = useState<Record<string, { ok: boolean; text: string }>>(
    {}
  );

  const handleTypeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const val = e.target.value as ProviderType;
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
    if (newKey.trim() && !isCliType(newType)) {
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
      const fetched = await fetchProviderModels(provider.id);
      setFetchNotes((prev) => ({
        ...prev,
        [provider.id]: {
          ok: true,
          text: t("settings.providers.fetched_models", { count: fetched.length }),
        },
      }));
    } catch (err) {
      setFetchNotes((prev) => ({
        ...prev,
        [provider.id]: { ok: false, text: errorMessage(err) },
      }));
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
                    <span className={styles.providerTypeTag}>
                      {t(`settings.providers.types.${p.provider_type}`, p.provider_type)}
                    </span>
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
                  {isCliType(p.provider_type) ? (
                    <div className={styles.cliHint}>
                      <Terminal size={16} aria-hidden="true" />
                      <p>{t(`settings.providers.cli_hint.${p.provider_type}`)}</p>
                    </div>
                  ) : (
                    <div className={styles.baseUrlWrap}>
                      <span className={styles.baseUrlLabel}>
                        {t("settings.providers.base_url")}:
                      </span>
                      <span className={styles.baseUrlValue}>{p.base_url}</span>
                    </div>
                  )}

                  {!isCliType(p.provider_type) && (
                    <SecretField
                      label={t("settings.providers.api_key_label")}
                      isStored={hasKey}
                      onTestConnection={() => testProvider(p.id)}
                      isTesting={testing}
                      onChange={(key) => setApiKey(p.id, key)}
                      onClear={() => clearApiKey(p.id)}
                    />
                  )}

                  {testResult && (
                    testResult.success ? (
                      <div className={styles.testResultSuccess}>
                        <PlugsConnected size={16} />
                        <span>
                          {isCliType(p.provider_type)
                            ? t("settings.providers.cli_found")
                            : t("settings.providers.test_success", {
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

                  <div className={styles.cardFooter}>
                    {isCliType(p.provider_type) ? (
                      <>
                        <Button
                          size="sm"
                          variant="secondary"
                          disabled={testing}
                          onClick={() => testProvider(p.id)}
                        >
                          {t("settings.providers.cli_check")}
                        </Button>
                        {CLI_INSTALL_PAGES[p.provider_type] && (
                          <Button
                            size="sm"
                            variant="ghost"
                            icon={<ArrowSquareOut size={14} />}
                            onClick={() => void ipc.openUrl(CLI_INSTALL_PAGES[p.provider_type])}
                          >
                            {t("settings.providers.cli_install")}
                          </Button>
                        )}
                      </>
                    ) : (
                      <>
                        <Button
                          size="sm"
                          variant="secondary"
                          icon={<ArrowClockwise size={14} />}
                          disabled={fetching || !hasKey}
                          disabledReason={!hasKey ? t("settings.providers.needs_key") : undefined}
                          onClick={() => handleFetchModels(p)}
                        >
                          {fetching
                            ? t("settings.providers.fetching_models")
                            : t("settings.providers.fetch_models_button")}
                        </Button>
                        {KEY_PAGES[p.id] && (
                          <Button
                            size="sm"
                            variant="ghost"
                            icon={<ArrowSquareOut size={14} />}
                            onClick={() => void ipc.openUrl(KEY_PAGES[p.id])}
                          >
                            {t("settings.providers.get_key")}
                          </Button>
                        )}
                      </>
                    )}
                  </div>
                  {fetchNotes[p.id] && (
                    <p
                      className={fetchNotes[p.id].ok ? styles.fetchNote : styles.fetchNoteError}
                      role="status"
                    >
                      {fetchNotes[p.id].text}
                    </p>
                  )}
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
            options={PROVIDER_TYPES.map((type) => ({
              value: type,
              label: t(`settings.providers.types.${type}`),
            }))}
          />

          <TextField
            label={t("settings.providers.name_label")}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder={t("settings.providers.name_placeholder")}
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

          {isCliType(newType) ? (
            <p className={styles.cliHintPlain}>{t(`settings.providers.cli_hint.${newType}`)}</p>
          ) : (
            <SecretField
              label={t("settings.providers.api_key_label")}
              isStored={false}
              value={newKey}
              onChange={setNewKey}
              placeholder="sk-..."
            />
          )}
        </div>
      </Dialog>
    </div>
  );
};
