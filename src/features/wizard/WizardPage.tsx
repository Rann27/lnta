import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { isTauri } from "@tauri-apps/api/core";
import { Sparkle, FloppyDisk } from "@phosphor-icons/react";
import { Button } from "../../components/ui/Button";
import { Switch } from "../../components/ui/Switch";
import { Checkbox } from "../../components/ui/Checkbox";
import { PathField } from "../../components/ui/PathField";
import { WizardStepper } from "./WizardStepper";
import { PreflightChecklist } from "./PreflightChecklist";
import { useWizardStore, PREFLIGHT_STEP } from "./store";
import { usePreflightActions } from "./usePreflightActions";
import { useProjectsStore } from "../projects/store";
import type { ModelReference, RoleConfig } from "../../lib/bindings";
import styles from "./WizardPage.module.css";

/** Folder used by the browser preview, which has no folder dialog. */
const PREVIEW_FOLDER = "D:/LN/Synthetic-Volume-1";

const ROLE_KEYS = ["translator", "repair", "qa_reviewer", "qa_fixer"] as const;

function modelLabel(ref: ModelReference): string {
  return `${ref.provider} / ${ref.model_id}`;
}

export const WizardPage: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const {
    currentStep,
    folderPath,
    profile,
    suggestedProfile,
    instructionFile,
    instructionCandidates,
    templateTreatment,
    sendFullGlossary,
    frozenMappings,
    isLoadingFrozen,
    toggleFrozenMapping,
    roles,
    preflight,
    isSaving,
    isPreflightRunning,
    error,
    initWizard,
    setStep,
    nextStep,
    prevStep,
    setProfile,
    setInstructionFile,
    setTemplateTreatment,
    setSendFullGlossary,
    runPreflightCheck,
    saveDraft,
    saveAndProceed,
  } = useWizardStore();
  const openProject = useProjectsStore((s) => s.openProject);

  const targetPath = searchParams.get("path") || (isTauri() ? "" : PREVIEW_FOLDER);

  useEffect(() => {
    if (!targetPath) {
      navigate("/");
      return;
    }
    // Read the folder again on every visit: files may have changed since the last one.
    void initWizard(targetPath);
  }, [targetPath, initWizard, navigate]);

  const hasBlockers = Boolean(
    preflight && preflight.items.some((i) => i.status === "blocker")
  );
  const itemStatus = (id: string) => preflight?.items.find((i) => i.id === id)?.status;
  const templateUsable = itemStatus("docx_template_valid") === "pass";
  // Without a report yet, offer the choice rather than hide it.
  const templateHasText = itemStatus("docx_template_contains_text") !== "pass";

  const actionHandlers = useMemo(
    () => ({
      projectPath: folderPath,
      rerun: runPreflightCheck,
      setInstructionFile: async (file: string) => {
        setInstructionFile(file);
        await saveDraft();
      },
      setProfile: async (next: string) => {
        await setProfile(next);
        await saveDraft();
      },
      focusInstruction: () => setStep(1),
      focusTemplateTreatment: () => setStep(2),
    }),
    [folderPath, runPreflightCheck, setInstructionFile, saveDraft, setProfile, setStep]
  );
  const preflightActions = usePreflightActions(actionHandlers);

  const handleFinish = async () => {
    try {
      await saveAndProceed();
      await openProject(folderPath);
      navigate("/workspace");
    } catch {
      // The store keeps the error and the footer shows it.
    }
  };

  const renderRole = (key: (typeof ROLE_KEYS)[number], role: RoleConfig | undefined) => {
    const primary = role?.primary;
    const hasPrimary = Boolean(primary && primary.provider && primary.model_id);
    const fallbacks = role?.fallbacks ?? [];
    return (
      <div key={key} className={styles.roleRow}>
        <span className={styles.roleName}>{t(`settings.roles.${key}.title`)}</span>
        <span className={hasPrimary ? styles.roleModel : styles.roleMissing}>
          {hasPrimary && primary ? modelLabel(primary) : t("wizard.role_not_set")}
        </span>
        <span className={styles.helper}>
          {fallbacks.length > 0
            ? t("wizard.fallbacks", { list: fallbacks.map(modelLabel).join(", ") })
            : t("wizard.no_fallbacks")}
        </span>
      </div>
    );
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return (
          <div className={styles.formGroup}>
            <label className={styles.label}>{t("wizard.steps.folder")}</label>
            <PathField value={folderPath} disabled />
            {!error && (
              <p className={styles.helper}>
                {preflight ? t("wizard.folder_ready") : t("wizard.preflight_checking")}
              </p>
            )}
          </div>
        );

      case 1:
        return (
          <div className={styles.formGroup}>
            <div className={styles.suggestionBadge}>
              <Sparkle size={14} aria-hidden="true" />
              <span>
                {t("wizard.suggested_profile", {
                  profile: suggestedProfile === "ja-en" ? "JP→EN" : "JP→ID",
                })}
              </span>
            </div>

            <label htmlFor="profile-select" className={styles.label}>
              {t("wizard.target_language")}
            </label>
            <select
              id="profile-select"
              className={styles.select}
              value={profile}
              onChange={(e) => void setProfile(e.target.value)}
            >
              <option value="ja-id">JP → ID (Bahasa Indonesia)</option>
              <option value="ja-en">JP → EN (English)</option>
            </select>

            <label htmlFor="instruction-select" className={styles.label} style={{ marginTop: "var(--space-3)" }}>
              {t("wizard.instruction_file")}
            </label>
            {instructionCandidates.length > 0 || instructionFile ? (
              <select
                id="instruction-select"
                className={styles.select}
                value={instructionFile}
                onChange={(e) => setInstructionFile(e.target.value)}
              >
                {instructionFile && !instructionCandidates.includes(instructionFile) && (
                  <option value={instructionFile}>
                    {t("wizard.instruction_custom", { file: instructionFile })}
                  </option>
                )}
                {instructionCandidates.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
            ) : (
              <p className={styles.helper}>{t("wizard.no_instruction")}</p>
            )}
            <div>
              <Button
                variant="secondary"
                size="sm"
                onClick={() =>
                  void preflightActions.handle("choose_instruction_file", {
                    id: "instruction_file",
                    status: "pass",
                    message_code: "",
                    params: {},
                    actions: [],
                  })
                }
              >
                {t("actions.choose_instruction_file")}
              </Button>
            </div>
          </div>
        );

      case 2:
        return (
          <div className={styles.formGroup}>
            {!templateUsable && preflight && (
              <div className={styles.formGroup}>
                <p className={styles.helper}>{t("wizard.template_missing_hint")}</p>
                <div className={styles.inlineActions}>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={async () => {
                      await preflightActions.handle("create_default_template", {
                        id: "docx_template_valid",
                        status: "blocker",
                        message_code: "",
                        params: {},
                        actions: [],
                      });
                    }}
                  >
                    {t("wizard.create_default_template_btn")}
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      void preflightActions.handle("choose_template_file", {
                        id: "docx_template_valid",
                        status: "blocker",
                        message_code: "",
                        params: {},
                        actions: [],
                      })
                    }
                  >
                    {t("actions.choose_template_file")}
                  </Button>
                </div>
              </div>
            )}
            {templateUsable && !templateHasText && (
              <p className={styles.helper}>{t("wizard.template_empty_hint")}</p>
            )}
            {templateHasText && (
            <>
            <label className={styles.label}>{t("wizard.template_treatment")}</label>
            <div className={styles.radioGroup}>
              {[
                { id: "keep_front_matter", labelKey: "wizard.treatment_keep" },
                { id: "ignore", labelKey: "wizard.treatment_ignore" },
                { id: "import_frozen", labelKey: "wizard.treatment_frozen" },
              ].map((opt) => (
                <label
                  key={opt.id}
                  className={[
                    styles.radioOption,
                    templateTreatment === opt.id ? styles.radioOptionSelected : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                >
                  <input
                    type="radio"
                    name="treatment"
                    value={opt.id}
                    checked={templateTreatment === opt.id}
                    onChange={() => setTemplateTreatment(opt.id)}
                  />
                  <span>{t(opt.labelKey)}</span>
                </label>
              ))}
            </div>

            {templateTreatment === "import_frozen" && (
              <div className={styles.frozenContainer}>
                <div className={styles.frozenHeader}>
                  <h3 className={styles.frozenTitle}>{t("wizard.frozen_mapping_title")}</h3>
                  <p className={styles.helper}>{t("wizard.frozen_mapping_desc")}</p>
                </div>

                {isLoadingFrozen ? (
                  <p className={styles.helper}>{t("wizard.frozen_loading")}</p>
                ) : frozenMappings.length === 0 ? (
                  <p className={styles.helper}>{t("wizard.frozen_empty")}</p>
                ) : (
                  <div className={styles.frozenTableWrapper}>
                    <table className={styles.frozenTable}>
                      <thead>
                        <tr>
                          <th style={{ width: "3rem", textAlign: "center" }}>
                            {t("wizard.frozen_th_freeze")}
                          </th>
                          <th>{t("wizard.frozen_th_chapter")}</th>
                          <th>{t("wizard.frozen_th_raw_title")}</th>
                          <th>{t("wizard.frozen_th_heading")}</th>
                          <th style={{ textAlign: "right" }}>{t("wizard.frozen_th_paragraphs")}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {frozenMappings.map((m) => (
                          <tr key={m.raw_chapter_prefix}>
                            <td style={{ textAlign: "center" }}>
                              <Checkbox
                                checked={m.is_frozen}
                                onChange={() => toggleFrozenMapping(m.raw_chapter_prefix)}
                                aria-label={`${t("wizard.frozen_th_freeze")} ${m.raw_chapter_prefix}`}
                              />
                            </td>
                            <td>
                              <strong>{m.raw_chapter_prefix}</strong>: {m.raw_chapter_file}
                            </td>
                            <td>{m.raw_chapter_title || "-"}</td>
                            <td>{m.docx_heading || "-"}</td>
                            <td style={{ textAlign: "right" }}>{m.paragraph_count}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </div>
            )}

            </>
            )}

            <div style={{ marginTop: "var(--space-4)" }}>
              <Switch
                label={t("wizard.send_full_glossary")}
                checked={sendFullGlossary}
                onChange={(e) => setSendFullGlossary(e.target.checked)}
              />
            </div>
          </div>
        );

      case 3:
        return (
          <div className={styles.formGroup}>
            <label className={styles.label}>{t("wizard.steps.model")}</label>
            <p className={styles.helper}>{t("wizard.roles_hint")}</p>
            <div className={styles.roleList}>
              {ROLE_KEYS.map((key) => renderRole(key, roles?.[key]))}
            </div>
            <div>
              <Button variant="secondary" size="sm" onClick={() => navigate("/settings")}>
                {t("wizard.open_settings")}
              </Button>
            </div>
          </div>
        );

      case 4:
      default:
        return (
          <div className={styles.formGroup}>
            <PreflightChecklist
              report={preflight}
              isLoading={isPreflightRunning}
              onRerun={runPreflightCheck}
              onAction={(actionId, item) => void preflightActions.handle(actionId, item)}
              availableActions={preflightActions.available}
            />
            {preflightActions.error && (
              <p className={styles.errorText} role="alert">
                {preflightActions.error}
              </p>
            )}
          </div>
        );
    }
  };

  return (
    <div className={styles.container}>
      <header className={styles.stepHeader}>
        <h1 className={styles.stepTitle}>{t("wizard.title")}</h1>
      </header>

      <div className={styles.layout}>
        <WizardStepper
          currentStep={currentStep}
          hasBlocker={hasBlockers}
          onSelectStep={setStep}
        />

        <div className={styles.stepContent}>
          {renderStepContent()}

          {error && (
            <p className={styles.errorText} role="alert">
              {error}
            </p>
          )}

          <footer className={styles.footer}>
            <div>
              {currentStep > 0 && (
                <Button variant="secondary" onClick={prevStep}>
                  {t("wizard.back")}
                </Button>
              )}
            </div>

            <div>
              {currentStep < PREFLIGHT_STEP ? (
                <Button variant="primary" onClick={nextStep} disabled={isSaving}>
                  {isSaving ? t("wizard.saving") : t("wizard.next")}
                </Button>
              ) : (
                <Button
                  variant="primary"
                  icon={<FloppyDisk size={16} />}
                  disabled={hasBlockers || isSaving}
                  disabledReason={hasBlockers ? t("wizard.run_disabled_reason") : undefined}
                  onClick={handleFinish}
                >
                  {isSaving ? t("wizard.saving") : t("wizard.save_and_open")}
                </Button>
              )}
            </div>
          </footer>
        </div>
      </div>
    </div>
  );
};
