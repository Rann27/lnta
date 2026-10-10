import React, { useState } from "react";
import { Plus, Warning } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { Dialog } from "../../components/ui/Dialog";
import { TextField } from "../../components/ui/TextField";
import { Select } from "../../components/ui/Select";
import { Checkbox } from "../../components/ui/Checkbox";
import { useSettingsStore } from "./store";
import type { ModelEntry } from "../../lib/bindings";
import styles from "./ModelsSection.module.css";

export const ModelsSection: React.FC = () => {
  const { t } = useTranslation();
  const { models, providers, registerModel } = useSettingsStore();

  const [isAddOpen, setIsAddOpen] = useState(false);
  const [provider, setProvider] = useState("");
  const [modelId, setModelId] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [contextWindow, setContextWindow] = useState("128000");
  const [maxOutputTokens, setMaxOutputTokens] = useState("4096");
  const [inputPrice, setInputPrice] = useState("2.5");
  const [outputPrice, setOutputPrice] = useState("10.0");
  const [supportsReasoning, setSupportsReasoning] = useState(false);

  const handleOpenAdd = () => {
    setProvider(providers[0]?.id ?? "openai");
    setModelId("");
    setDisplayName("");
    setContextWindow("128000");
    setMaxOutputTokens("4096");
    setInputPrice("2.5");
    setOutputPrice("10.0");
    setSupportsReasoning(false);
    setIsAddOpen(true);
  };

  const handleSaveModel = async () => {
    if (!modelId.trim()) return;

    const entry: ModelEntry = {
      provider: provider.trim(),
      model_id: modelId.trim(),
      display_name: displayName.trim() || modelId.trim(),
      context_window: parseInt(contextWindow, 10) || 128000,
      max_output_tokens: parseInt(maxOutputTokens, 10) || 0,
      pricing: {
        input_per_million: parseFloat(inputPrice) || 0,
        cached_input_per_million: 0,
        output_per_million: parseFloat(outputPrice) || 0,
      },
      value_source: "user_supplied",
      supports_reasoning: supportsReasoning,
    };

    await registerModel(entry);
    setIsAddOpen(false);
  };

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <div className={styles.titleGroup}>
          <h2 className={styles.title}>{t("settings.models.title")}</h2>
          <p className={styles.subtitle}>{t("settings.models.subtitle")}</p>
        </div>

        <Button
          variant="primary"
          icon={<Plus size={16} />}
          onClick={handleOpenAdd}
        >
          {t("settings.models.add_button")}
        </Button>
      </header>

      {models.length === 0 ? (
        <div className={styles.emptyState}>
          <p>{t("settings.models.empty_text")}</p>
          <Button variant="secondary" onClick={handleOpenAdd}>
            {t("settings.models.add_button")}
          </Button>
        </div>
      ) : (
        <div className={styles.modelGrid}>
          {models.map((m) => {
            const isMissingOutput = !m.max_output_tokens || m.max_output_tokens <= 0;

            return (
              <div key={`${m.provider}:${m.model_id}`} className={styles.modelCard}>
                <div className={styles.cardTop}>
                  <div className={styles.nameWrap}>
                    <span className={styles.displayName}>{m.display_name}</span>
                    <span className={styles.modelId}>{m.model_id}</span>
                  </div>
                  <span className={styles.providerTag}>{m.provider}</span>
                </div>

                <div className={styles.cardBottom}>
                  <div className={styles.specs}>
                    <span>
                      {t("model_picker.context_tokens", {
                        count: m.context_window,
                      })}
                    </span>

                    {isMissingOutput ? (
                      <span className={styles.warningOutput}>
                        <Warning size={14} />
                        <span>{t("model_picker.missing_output_limit")}</span>
                      </span>
                    ) : (
                      <span>
                        {t("model_picker.output_tokens", {
                          count: m.max_output_tokens,
                        })}
                      </span>
                    )}

                    {m.supports_reasoning && (
                      <span>{t("settings.models.reasoning_supported")}</span>
                    )}
                  </div>

                  <div className={styles.pricing}>
                    {m.pricing?.input_per_million != null
                      ? `$${m.pricing.input_per_million.toFixed(2)} in`
                      : "- in"}{" "}
                    /{" "}
                    {m.pricing?.output_per_million != null
                      ? `$${m.pricing.output_per_million.toFixed(2)} out`
                      : "- out"}{" "}
                    per 1M
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Add Model Dialog */}
      <Dialog
        isOpen={isAddOpen}
        onClose={() => setIsAddOpen(false)}
        title={t("settings.models.dialog_title")}
        actions={
          <>
            <Button variant="secondary" onClick={() => setIsAddOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="primary" onClick={handleSaveModel}>
              {t("common.save")}
            </Button>
          </>
        }
      >
        <div className={styles.dialogForm}>
          {providers.length > 0 ? (
            <Select
              label={t("settings.models.provider_label")}
              value={provider}
              onChange={(e) => setProvider(e.target.value)}
              options={providers.map((p) => ({
                value: p.id,
                label: `${p.name} (${p.id})`,
              }))}
            />
          ) : (
            <TextField
              label={t("settings.models.provider_label")}
              value={provider}
              onChange={(e) => setProvider(e.target.value)}
              placeholder="openai"
            />
          )}

          <TextField
            label={t("settings.models.model_id_label")}
            value={modelId}
            onChange={(e) => setModelId(e.target.value)}
            placeholder="gpt-4o"
          />

          <TextField
            label={t("settings.models.display_name_label")}
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="GPT-4o"
          />

          <div className={styles.gridTwo}>
            <TextField
              label={t("settings.models.context_window_label")}
              type="number"
              value={contextWindow}
              onChange={(e) => setContextWindow(e.target.value)}
            />

            <TextField
              label={t("settings.models.max_output_label")}
              type="number"
              value={maxOutputTokens}
              onChange={(e) => setMaxOutputTokens(e.target.value)}
              hint={t("settings.models.max_output_hint")}
            />
          </div>

          <div className={styles.gridTwo}>
            <TextField
              label={t("settings.models.input_price_label")}
              type="number"
              step="0.1"
              value={inputPrice}
              onChange={(e) => setInputPrice(e.target.value)}
            />

            <TextField
              label={t("settings.models.output_price_label")}
              type="number"
              step="0.1"
              value={outputPrice}
              onChange={(e) => setOutputPrice(e.target.value)}
            />
          </div>

          <Checkbox
            label={t("settings.models.supports_reasoning_label")}
            checked={supportsReasoning}
            onChange={(e) => setSupportsReasoning(e.target.checked)}
          />
        </div>
      </Dialog>
    </div>
  );
};
