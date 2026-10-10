import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Sparkle, Play } from "@phosphor-icons/react";
import { Button } from "../../components/ui/Button";
import { Switch } from "../../components/ui/Switch";
import { Checkbox } from "../../components/ui/Checkbox";
import { PathField } from "../../components/ui/PathField";
import { WizardStepper } from "./WizardStepper";
import { PreflightChecklist } from "./PreflightChecklist";
import { useWizardStore } from "./store";
import styles from "./WizardPage.module.css";

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
    preflight,
    isPreflightRunning,
    initWizard,
    setStep,
    nextStep,
    prevStep,
    setProfile,
    setInstructionFile,
    setTemplateTreatment,
    setSendFullGlossary,
    runPreflightCheck,
    saveAndProceed,
  } = useWizardStore();

  useEffect(() => {
    const pathFromQuery = searchParams.get("path") || "D:/LN/Synthetic-Volume-1";
    if (!preflight || folderPath !== pathFromQuery) {
      void initWizard(pathFromQuery);
    }
  }, [searchParams, folderPath, preflight, initWizard]);

  const hasBlockers = Boolean(
    preflight && preflight.items.some((i) => i.status === "blocker")
  );

  const handleFinish = async () => {
    try {
      await saveAndProceed();
      navigate("/workspace");
    } catch {
      // save error
    }
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return (
          <div className={styles.formGroup}>
            <label className={styles.label}>{t("wizard.steps.folder")}</label>
            <PathField value={folderPath} disabled />
            <p className={styles.helper}>
              {preflight
                ? t("preflight.ready", "Folder siap diperiksa dan dikonfigurasi.")
                : t("wizard.preflight_checking")}
            </p>
          </div>
        );

      case 1:
        return (
          <div className={styles.formGroup}>
            <div className={styles.suggestionBadge}>
              <Sparkle size={14} aria-hidden="true" />
              <span>
                {t("wizard.suggested_profile", { profile: suggestedProfile })}
              </span>
            </div>

            <label htmlFor="profile-select" className={styles.label}>
              {t("wizard.target_language")}
            </label>
            <select
              id="profile-select"
              className={styles.select}
              value={profile}
              onChange={(e) => setProfile(e.target.value)}
            >
              <option value="ja-id">JP → ID (Bahasa Indonesia)</option>
              <option value="ja-en">JP → EN (English)</option>
            </select>

            <label htmlFor="instruction-select" className={styles.label} style={{ marginTop: "var(--space-3)" }}>
              {t("wizard.instruction_file")}
            </label>
            <select
              id="instruction-select"
              className={styles.select}
              value={instructionFile}
              onChange={(e) => setInstructionFile(e.target.value)}
            >
              {instructionCandidates.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </select>
          </div>
        );

      case 2:
        return (
          <div className={styles.formGroup}>
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
            <div className={styles.radioGroup}>
              <div className={styles.radioOption}>
                <strong>Translator:</strong> Claude 3.5 Sonnet (fallback: GPT-4o)
              </div>
              <div className={styles.radioOption}>
                <strong>Repair:</strong> Claude 3.5 Sonnet (fallback: GPT-4o)
              </div>
              <div className={styles.radioOption}>
                <strong>QA Reviewer:</strong> GPT-4o (fallback: Gemini 1.5 Pro)
              </div>
              <div className={styles.radioOption}>
                <strong>QA Fixer:</strong> Claude 3.5 Sonnet (fallback: GPT-4o)
              </div>
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
            />
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

          <footer className={styles.footer}>
            <div>
              {currentStep > 0 && (
                <Button variant="secondary" onClick={prevStep}>
                  {t("wizard.back")}
                </Button>
              )}
            </div>

            <div>
              {currentStep < 4 ? (
                <Button variant="primary" onClick={nextStep}>
                  {t("wizard.next")}
                </Button>
              ) : (
                <Button
                  variant="primary"
                  icon={<Play size={16} />}
                  disabled={hasBlockers}
                  disabledReason={hasBlockers ? t("wizard.run_disabled_reason") : undefined}
                  onClick={handleFinish}
                >
                  {t("actions.run_volume")}
                </Button>
              )}
            </div>
          </footer>
        </div>
      </div>
    </div>
  );
};
