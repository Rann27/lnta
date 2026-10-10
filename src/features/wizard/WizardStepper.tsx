import React from "react";
import { Check, Warning } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import styles from "./WizardStepper.module.css";

export interface WizardStepperProps {
  currentStep: number;
  hasBlocker?: boolean;
  onSelectStep: (step: number) => void;
}

const STEP_KEYS = [
  "wizard.steps.folder",
  "wizard.steps.profile",
  "wizard.steps.template",
  "wizard.steps.model",
  "wizard.steps.preflight",
] as const;

export const WizardStepper: React.FC<WizardStepperProps> = ({
  currentStep,
  hasBlocker = false,
  onSelectStep,
}) => {
  const { t } = useTranslation();

  return (
    <ol className={styles.stepper} aria-label={t("wizard.title")}>
      {STEP_KEYS.map((key, index) => {
        const isActive = index === currentStep;
        const isCompleted = index < currentStep;
        const isPreflight = index === 4;
        const showWarning = isPreflight && hasBlocker;

        return (
          <li key={key}>
            <button
              type="button"
              className={[
                styles.step,
                isActive ? styles.active : "",
                isCompleted ? styles.completed : "",
                showWarning ? styles.hasBlocker : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => onSelectStep(index)}
              aria-current={isActive ? "step" : undefined}
            >
              <span className={styles.icon} aria-hidden="true">
                {showWarning ? (
                  <Warning size={14} />
                ) : isCompleted ? (
                  <Check size={14} />
                ) : (
                  <span>{index + 1}</span>
                )}
              </span>
              <span className={styles.label}>{t(key)}</span>
            </button>
          </li>
        );
      })}
    </ol>
  );
};
