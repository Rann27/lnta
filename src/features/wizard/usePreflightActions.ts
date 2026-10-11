import { useCallback, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ipc } from "../../lib/ipc";
import { errorMessage } from "../../lib/errors";
import type { PreflightItem } from "../../lib/bindings";

/** Hooks a screen offers so preflight actions can change its own state. */
export interface PreflightActionHandlers {
  projectPath: string;
  rerun: () => Promise<void> | void;
  setInstructionFile?: (file: string) => Promise<void> | void;
  setProfile?: (profile: string) => Promise<void> | void;
  focusInstruction?: () => void;
  focusTemplateTreatment?: () => void;
  openGlossary?: () => void;
  openPlan?: () => void;
}

/** Path of `file` relative to `root` when it lives inside the project folder. */
export function relativeToProject(root: string, file: string): string {
  const norm = (p: string) => p.replace(/\\/g, "/").replace(/\/+$/, "");
  const base = norm(root);
  const target = norm(file);
  if (target.toLowerCase().startsWith(`${base.toLowerCase()}/`)) {
    return target.slice(base.length + 1);
  }
  return file;
}

async function pickFile(title: string, extension: string, defaultPath: string) {
  if (!isTauri()) return null;
  const selected = await open({
    multiple: false,
    directory: false,
    title,
    defaultPath,
    filters: [{ name: extension, extensions: [extension] }],
  });
  return typeof selected === "string" ? selected : null;
}

/**
 * Runs the actions attached to preflight items. `available` lists only the actions this
 * screen can carry out, so the checklist never shows a button that does nothing.
 */
export function usePreflightActions(handlers: PreflightActionHandlers) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const available = useMemo(() => {
    const ids = new Set([
      "create_default_template",
      "choose_template_file",
      "open_provider_settings",
      "open_model_settings",
      "open_other_folder",
      "fix_macrons",
      "view_missing_images",
      "dismiss",
    ]);
    if (handlers.setInstructionFile) ids.add("choose_instruction_file");
    if (handlers.setProfile) ids.add("switch_profile");
    if (handlers.focusInstruction) ids.add("select_instruction_candidate");
    if (handlers.focusTemplateTreatment) ids.add("choose_template_treatment");
    if (handlers.openGlossary) ids.add("open_glossary_tab");
    if (handlers.openPlan) ids.add("view_split_plan");
    return ids;
  }, [handlers]);

  const handle = useCallback(
    async (actionId: string, item: PreflightItem) => {
      const { projectPath } = handlers;
      setError(null);
      setBusy(true);
      try {
        switch (actionId) {
          case "create_default_template":
            await ipc.createDefaultTemplate(projectPath);
            await handlers.rerun();
            break;
          case "choose_template_file": {
            const file = await pickFile(t("actions.choose_template_file"), "docx", projectPath);
            if (file) {
              await ipc.importTemplate(projectPath, file);
              await handlers.rerun();
            }
            break;
          }
          case "choose_instruction_file": {
            const file = await pickFile(t("actions.choose_instruction_file"), "md", projectPath);
            if (file && handlers.setInstructionFile) {
              await handlers.setInstructionFile(relativeToProject(projectPath, file));
              await handlers.rerun();
            }
            break;
          }
          case "switch_profile": {
            const detected = item.params.detected_profile;
            if (detected && handlers.setProfile) {
              await handlers.setProfile(detected);
              await handlers.rerun();
            }
            break;
          }
          case "fix_macrons":
            await ipc.fixGlossaryMacrons(projectPath);
            await handlers.rerun();
            break;
          case "view_missing_images":
            await ipc.revealPath(`${projectPath.replace(/[\\/]+$/, "")}/illustrations`);
            break;
          case "open_provider_settings":
          case "open_model_settings":
            navigate("/settings");
            break;
          case "open_other_folder":
            navigate("/");
            break;
          case "select_instruction_candidate":
            handlers.focusInstruction?.();
            break;
          case "choose_template_treatment":
            handlers.focusTemplateTreatment?.();
            break;
          case "open_glossary_tab":
            handlers.openGlossary?.();
            break;
          case "view_split_plan":
            handlers.openPlan?.();
            break;
          default:
            break;
        }
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setBusy(false);
      }
    },
    [handlers, navigate, t]
  );

  return { available, handle, error, busy };
}
