import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { FolderOpen, FloppyDisk, Trash } from "@phosphor-icons/react";
import { Button } from "../../components/ui/Button";
import { Dialog } from "../../components/ui/Dialog";
import { NumberField } from "../../components/ui/NumberField";
import { Select } from "../../components/ui/Select";
import { Switch } from "../../components/ui/Switch";
import { PreflightChecklist } from "../wizard/PreflightChecklist";
import { usePreflightActions } from "../wizard/usePreflightActions";
import { useWorkspaceStore } from "../workspace/store";
import { useProjectsStore } from "../projects/store";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errors";
import type { ProjectConfig } from "../../lib/bindings";
import styles from "./ProjectTab.module.css";

const PROFILES = [
  { value: "ja-id", label: "JP→ID (Bahasa Indonesia)" },
  { value: "ja-en", label: "JP→EN (English)" },
];

function folderName(path: string): string {
  const parts = path.replace(/[\\/]+$/, "").split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** Per-volume settings, detected files, preflight, and reset (PRD §11.2 Workspace: Proyek). */
export const ProjectTab: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const projectPath = useWorkspaceStore((s) => s.projectPath);
  const chapters = useWorkspaceStore((s) => s.chapters);
  const imagePageIds = useWorkspaceStore((s) => s.imagePageIds);
  const glossaryCount = useWorkspaceStore((s) => s.glossaryCount);
  const preflight = useWorkspaceStore((s) => s.preflight);
  const runActive = useWorkspaceStore((s) => s.runState.active);
  const initWorkspace = useWorkspaceStore((s) => s.initWorkspace);
  const setActiveTab = useWorkspaceStore((s) => s.setActiveTab);
  const resetActiveProject = useProjectsStore((s) => s.resetActiveProject);

  const [saved, setSaved] = useState<ProjectConfig | null>(null);
  const [draft, setDraft] = useState<ProjectConfig | null>(null);
  const [candidates, setCandidates] = useState<string[]>([]);
  const [preflightRunning, setPreflightRunning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmProfile, setConfirmProfile] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);

  useEffect(() => {
    if (!projectPath) return;
    let cancelled = false;
    ipc
      .getProjectConfig(projectPath)
      .then((cfg) => {
        if (cancelled) return;
        setSaved(cfg);
        setDraft(cfg);
      })
      .catch((err) => !cancelled && setError(errorMessage(err)));
    return () => {
      cancelled = true;
    };
  }, [projectPath]);

  const profile = draft?.profile ?? "ja-id";
  useEffect(() => {
    if (!projectPath) return;
    let cancelled = false;
    ipc
      .listInstructionCandidates(projectPath, profile)
      .then((list) => !cancelled && setCandidates(list))
      .catch(() => !cancelled && setCandidates([]));
    return () => {
      cancelled = true;
    };
  }, [projectPath, profile]);

  const rerun = useCallback(async () => {
    if (!projectPath) return;
    setPreflightRunning(true);
    try {
      const report = await ipc.runPreflight(projectPath);
      useWorkspaceStore.setState({ preflight: report });
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPreflightRunning(false);
    }
  }, [projectPath]);

  const persist = useCallback(
    async (next: ProjectConfig) => {
      if (!projectPath || !saved) return;
      setSaving(true);
      setError(null);
      try {
        await ipc.saveProjectConfig(projectPath, next);
        const reparse =
          next.profile !== saved.profile ||
          next.normalize_ruby !== saved.normalize_ruby ||
          next.strip_title_brackets !== saved.strip_title_brackets;
        setSaved(next);
        setDraft(next);
        setNotice(t("project_tab.saved"));
        if (reparse) {
          await initWorkspace(projectPath);
        } else {
          await rerun();
        }
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setSaving(false);
      }
    },
    [projectPath, saved, initWorkspace, rerun, t]
  );

  const actionHandlers = useMemo(
    () => ({
      projectPath,
      rerun,
      setInstructionFile: async (file: string) => {
        if (saved) await persist({ ...saved, instruction_file: file });
      },
      setProfile: async (next: string) => {
        if (saved) {
          await persist({
            ...saved,
            profile: next,
            target_language: next === "ja-en" ? "en" : "id",
            instruction_file: null,
          });
        }
      },
      focusInstruction: () => document.getElementById("project-instruction")?.focus(),
      focusTemplateTreatment: () => document.getElementById("project-treatment")?.focus(),
      openGlossary: () => setActiveTab("glossary"),
    }),
    [projectPath, rerun, saved, persist, setActiveTab]
  );
  const preflightActions = usePreflightActions(actionHandlers);

  if (!draft || !saved) {
    return (
      <div className={styles.container}>
        <p className={styles.hint}>{error ?? t("project_tab.loading")}</p>
      </div>
    );
  }

  const dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  const locked = runActive || saving;
  const doneCount = chapters.filter((c) => c.status === "done").length;
  const imageCount = chapters.filter((c) => imagePageIds[c.id]).length;
  const customInstruction =
    draft.instruction_file && !candidates.includes(draft.instruction_file)
      ? draft.instruction_file
      : null;
  const item = (id: string) => preflight?.items.find((i) => i.id === id);
  const itemText = (id: string) => {
    const found = item(id);
    return found ? t(found.message_code, found.params as Record<string, string>) : "-";
  };

  const update = <K extends keyof ProjectConfig>(key: K, value: ProjectConfig[K]) => {
    setNotice(null);
    setDraft((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const changeProfile = (next: string) => {
    setNotice(null);
    setDraft((prev) =>
      prev
        ? {
            ...prev,
            profile: next,
            target_language: next === "ja-en" ? "en" : "id",
            // An auto-detected instruction follows the profile; a hand-picked one stays.
            instruction_file: customInstruction,
          }
        : prev
    );
  };

  const handleSave = () => {
    if (draft.profile !== saved.profile && doneCount > 0) {
      setConfirmProfile(true);
      return;
    }
    void persist(draft);
  };

  const handleReset = async () => {
    setConfirmReset(false);
    try {
      await ipc.resetProject(projectPath);
      resetActiveProject();
      useWorkspaceStore.setState({ projectPath: "" });
      navigate("/");
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const treatmentOptions = [
    { value: "keep_front_matter", label: t("wizard.treatment_keep") },
    { value: "ignore", label: t("wizard.treatment_ignore") },
    { value: "frozen_prefix", label: t("project_tab.treatment_frozen_prefix") },
  ];
  if (!treatmentOptions.some((o) => o.value === draft.template_treatment)) {
    // Older projects store "copy", which renders like "ignore".
    treatmentOptions.push({ value: draft.template_treatment, label: t("wizard.treatment_ignore") });
  }

  return (
    <div className={styles.container}>
      <section className={styles.section} aria-labelledby="project-files-title">
        <h2 id="project-files-title" className={styles.sectionTitle}>
          {t("project_tab.files_title")}
        </h2>
        <dl className={styles.facts}>
          <dt>{t("project_tab.folder_label")}</dt>
          <dd className={styles.factWithAction}>
            <span className={styles.path} title={projectPath}>
              {projectPath}
            </span>
            <Button
              size="sm"
              variant="ghost"
              icon={<FolderOpen size={14} />}
              onClick={() => void ipc.revealPath(projectPath)}
            >
              {t("project_tab.reveal_folder")}
            </Button>
          </dd>
          <dt>{t("project_tab.chapters_label")}</dt>
          <dd>
            {t("project_tab.chapters_value", { count: chapters.length })},{" "}
            {t("project_tab.image_pages_value", { count: imageCount })}
          </dd>
          <dt>{t("project_tab.template_label")}</dt>
          <dd>{itemText("docx_template_valid")}</dd>
          <dt>{t("project_tab.glossary_label")}</dt>
          <dd>
            {glossaryCount !== null
              ? t("project_tab.glossary_value", { count: glossaryCount })
              : itemText("glossary_file_exists")}
          </dd>
        </dl>

        <div className={styles.fieldGrid}>
          <Select
            id="project-profile"
            label={t("project_tab.profile_label")}
            value={draft.profile}
            options={PROFILES}
            disabled={locked}
            onChange={(e) => changeProfile(e.target.value)}
          />
          <div className={styles.fieldWithAction}>
            <Select
              id="project-instruction"
              label={t("project_tab.instruction_label")}
              value={draft.instruction_file ?? ""}
              disabled={locked}
              options={[
                {
                  value: "",
                  label: candidates[0]
                    ? t("project_tab.instruction_auto", { file: candidates[0] })
                    : t("project_tab.instruction_auto_none"),
                },
                ...(customInstruction ? [{ value: customInstruction, label: customInstruction }] : []),
                ...candidates.map((c) => ({ value: c, label: c })),
              ]}
              onChange={(e) => update("instruction_file", e.target.value || null)}
            />
            <Button
              size="sm"
              variant="secondary"
              disabled={locked}
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
      </section>

      <section className={styles.section} aria-labelledby="project-settings-title">
        <h2 id="project-settings-title" className={styles.sectionTitle}>
          {t("project_tab.settings_title")}
        </h2>
        <p className={styles.hint}>
          {runActive ? t("project_tab.locked") : t("project_tab.settings_hint")}
        </p>
        <div className={styles.fieldGrid}>
          <NumberField
            label={t("project_tab.chunk_threshold")}
            hint={t("project_tab.chunk_threshold_hint")}
            value={draft.chunk_threshold_lines}
            min={50}
            disabled={locked}
            onChange={(v) => v !== undefined && update("chunk_threshold_lines", Math.round(v))}
          />
          <NumberField
            label={t("project_tab.context_k")}
            value={draft.context_k_segments}
            min={0}
            disabled={locked}
            onChange={(v) => v !== undefined && update("context_k_segments", Math.round(v))}
          />
          <Select
            id="project-treatment"
            label={t("wizard.template_treatment")}
            value={draft.template_treatment}
            options={treatmentOptions}
            disabled={locked}
            onChange={(e) => update("template_treatment", e.target.value)}
          />
          <NumberField
            label={t("project_tab.budget_tokens")}
            hint={t("project_tab.budget_hint")}
            value={draft.budget_token_limit ?? undefined}
            min={0}
            disabled={locked}
            onChange={(v) => update("budget_token_limit", v === undefined ? null : Math.round(v))}
          />
          <NumberField
            label={t("project_tab.budget_cost")}
            value={draft.budget_cost_limit ?? undefined}
            min={0}
            step={0.01}
            disabled={locked}
            onChange={(v) => update("budget_cost_limit", v === undefined ? null : v)}
          />
        </div>
        <div className={styles.switches}>
          <Switch
            label={t("wizard.send_full_glossary")}
            checked={draft.send_full_glossary}
            disabled={locked}
            onChange={(e) => update("send_full_glossary", e.target.checked)}
          />
          <Switch
            label={t("project_tab.strip_title_brackets")}
            checked={draft.strip_title_brackets}
            disabled={locked}
            onChange={(e) => update("strip_title_brackets", e.target.checked)}
          />
          <Switch
            label={t("project_tab.normalize_ruby")}
            checked={draft.normalize_ruby}
            disabled={locked}
            onChange={(e) => update("normalize_ruby", e.target.checked)}
          />
        </div>
        <div className={styles.formActions}>
          <Button
            variant="secondary"
            icon={<FloppyDisk size={16} />}
            disabled={!dirty || locked}
            onClick={handleSave}
          >
            {saving ? t("wizard.saving") : t("common.save")}
          </Button>
          <Button variant="ghost" disabled={!dirty || saving} onClick={() => setDraft(saved)}>
            {t("project_tab.discard")}
          </Button>
          <span className={styles.status} role="status">
            {notice}
          </span>
        </div>
        {error && (
          <p className={styles.error} role="alert">
            {error}
          </p>
        )}
      </section>

      <section className={styles.section} aria-labelledby="project-preflight-title">
        <h2 id="project-preflight-title" className={styles.sectionTitle}>
          {t("project_tab.preflight_title")}
        </h2>
        <PreflightChecklist
          report={preflight}
          isLoading={preflightRunning}
          onRerun={() => void rerun()}
          onAction={(actionId, it) => void preflightActions.handle(actionId, it)}
          availableActions={preflightActions.available}
        />
        {preflightActions.error && (
          <p className={styles.error} role="alert">
            {preflightActions.error}
          </p>
        )}
      </section>

      <section className={styles.section} aria-labelledby="project-reset-title">
        <h2 id="project-reset-title" className={styles.sectionTitle}>
          {t("project_tab.reset_title")}
        </h2>
        <p className={styles.hint}>{t("project_tab.reset_hint")}</p>
        <div>
          <Button
            variant="danger"
            icon={<Trash size={16} />}
            disabled={runActive}
            disabledReason={runActive ? t("project_tab.reset_locked") : undefined}
            onClick={() => setConfirmReset(true)}
          >
            {t("project_tab.reset_button")}
          </Button>
        </div>
      </section>

      <Dialog
        isOpen={confirmProfile}
        onClose={() => setConfirmProfile(false)}
        title={t("project_tab.profile_confirm_title")}
        description={t("project_tab.profile_confirm_body", { count: doneCount })}
        actions={
          <>
            <Button variant="secondary" onClick={() => setConfirmProfile(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                setConfirmProfile(false);
                void persist(draft);
              }}
            >
              {t("project_tab.profile_confirm_action")}
            </Button>
          </>
        }
      />

      <Dialog
        isOpen={confirmReset}
        onClose={() => setConfirmReset(false)}
        variant="destructive"
        title={t("project_tab.reset_confirm_title")}
        description={t("project_tab.reset_confirm_body", { name: folderName(projectPath) })}
        actions={
          <>
            <Button variant="secondary" onClick={() => setConfirmReset(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="danger" onClick={() => void handleReset()}>
              {t("project_tab.reset_confirm_action")}
            </Button>
          </>
        }
      />
    </div>
  );
};
