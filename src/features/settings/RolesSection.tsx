import React, { useState } from "react";
import { CaretUp, CaretDown, Trash, Plus } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { ModelPicker } from "../../components/ui/ModelPicker";
import { useSettingsStore } from "./store";
import type { ModelReference, RolesConfig } from "../../lib/bindings";
import styles from "./RolesSection.module.css";

const ROLE_KEYS: (keyof RolesConfig)[] = [
  "translator",
  "repair",
  "qa_reviewer",
  "qa_fixer",
];

export const RolesSection: React.FC = () => {
  const { t } = useTranslation();
  const {
    globalSettings,
    models,
    updateRolePrimary,
    addRoleFallback,
    removeRoleFallback,
    reorderRoleFallback,
  } = useSettingsStore();

  const [pendingFallback, setPendingFallback] = useState<
    Record<keyof RolesConfig, ModelReference | null>
  >({
    translator: null,
    repair: null,
    qa_reviewer: null,
    qa_fixer: null,
  });

  if (!globalSettings) {
    return null;
  }

  const handleAddFallback = async (roleKey: keyof RolesConfig) => {
    const candidate = pendingFallback[roleKey];
    if (!candidate) return;
    await addRoleFallback(roleKey, candidate);
    setPendingFallback((prev) => ({ ...prev, [roleKey]: null }));
  };

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <h2 className={styles.title}>{t("settings.roles.title")}</h2>
        <p className={styles.subtitle}>{t("settings.roles.subtitle")}</p>
      </header>

      <div className={styles.rolesGrid}>
        {ROLE_KEYS.map((roleKey) => {
          const roleConfig = globalSettings.roles[roleKey];
          if (!roleConfig) return null;

          return (
            <div key={roleKey} className={styles.roleCard}>
              <div className={styles.roleHeader}>
                <h3 className={styles.roleTitle}>{t(`settings.roles.${roleKey}.title`)}</h3>
                <span className={styles.roleDesc}>{t(`settings.roles.${roleKey}.desc`)}</span>
              </div>

              {/* Primary Model */}
              <div className={styles.sectionBlock}>
                <label className={styles.blockLabel}>
                  {t("settings.roles.primary_label")}
                </label>
                <ModelPicker
                  value={roleConfig.primary}
                  models={models}
                  onChange={(val) => updateRolePrimary(roleKey, val)}
                />
              </div>

              {/* Fallback Models List */}
              <div className={styles.sectionBlock}>
                <label className={styles.blockLabel}>
                  {t("settings.roles.fallbacks_label")}
                </label>

                {roleConfig.fallbacks.length === 0 ? (
                  <div className={styles.emptyFallbacks}>
                    {t("settings.roles.no_fallbacks")}
                  </div>
                ) : (
                  <div className={styles.fallbacksList}>
                    {roleConfig.fallbacks.map((fb, idx) => {
                      const matched = models.find(
                        (m) => m.provider === fb.provider && m.model_id === fb.model_id
                      );

                      return (
                        <div
                          key={`${fb.provider}:${fb.model_id}:${idx}`}
                          className={styles.fallbackRow}
                        >
                          <div className={styles.fallbackInfo}>
                            <span className={styles.fallbackIndex}>#{idx + 1}</span>
                            <span className={styles.fallbackName}>
                              {matched?.display_name ?? fb.model_id}
                            </span>
                            <span className={styles.fallbackModelId}>
                              ({fb.provider})
                            </span>
                          </div>

                          <div className={styles.fallbackActions}>
                            <Button
                              size="sm"
                              variant="ghost"
                              disabled={idx === 0}
                              icon={<CaretUp size={14} />}
                              onClick={() =>
                                reorderRoleFallback(roleKey, idx, idx - 1)
                              }
                              aria-label={t("settings.roles.move_up")}
                            />
                            <Button
                              size="sm"
                              variant="ghost"
                              disabled={idx === roleConfig.fallbacks.length - 1}
                              icon={<CaretDown size={14} />}
                              onClick={() =>
                                reorderRoleFallback(roleKey, idx, idx + 1)
                              }
                              aria-label={t("settings.roles.move_down")}
                            />
                            <Button
                              size="sm"
                              variant="ghost"
                              icon={<Trash size={14} />}
                              onClick={() => removeRoleFallback(roleKey, idx)}
                              aria-label={t("common.delete")}
                            />
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* Add Fallback Row */}
                <div className={styles.addFallbackRow}>
                  <div className={styles.addFallbackPicker}>
                    <ModelPicker
                      placeholder={t("settings.roles.select_fallback_placeholder")}
                      value={pendingFallback[roleKey]}
                      models={models}
                      onChange={(val) =>
                        setPendingFallback((prev) => ({
                          ...prev,
                          [roleKey]: val,
                        }))
                      }
                    />
                  </div>
                  <Button
                    size="md"
                    variant="secondary"
                    icon={<Plus size={16} />}
                    disabled={!pendingFallback[roleKey]}
                    onClick={() => handleAddFallback(roleKey)}
                  >
                    {t("settings.roles.add_fallback_button")}
                  </Button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
