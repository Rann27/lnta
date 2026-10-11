import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Banner } from "../../../components/ui/Banner";
import { Button } from "../../../components/ui/Button";
import { useWorkspaceStore } from "../store";

export const RunTabBanners: React.FC = () => {
  const { t } = useTranslation();
  const {
    preflight,
    docxStatus,
    chapters,
    selectChapter,
    setActiveTab,
    retryDocxRender,
    runError,
  } = useWorkspaceStore();

  const [dismissedDocx, setDismissedDocx] = useState(false);
  const [dismissedChapters, setDismissedChapters] = useState<Record<string, boolean>>({});

  interface BannerItem {
    id: string;
    variant: "blocker" | "attention";
    title: string;
    description: string;
    action: React.ReactNode;
    onClose?: () => void;
  }

  const bannerList: BannerItem[] = [];

  // 1. Preflight Blocker (never dismissible)
  const hasBlockers = preflight ? !preflight.can_run || preflight.items.some((i) => i.status === "blocker") : false;
  if (hasBlockers) {
    bannerList.push({
      id: "preflight_blocker",
      variant: "blocker",
      title: t("workspace.run_tab.blocker_preflight_title"),
      description: t("workspace.run_tab.blocker_preflight_desc"),
      action: (
        <Button
          variant="secondary"
          onClick={() => setActiveTab("project")}
        >
          {t("workspace.run_tab.action_check_preflight")}
        </Button>
      ),
    });
  }

  // 2. A run or command the backend refused, e.g. a missing instruction file
  if (runError) {
    bannerList.push({
      id: "run_error",
      variant: "blocker",
      title: t("workspace.run_error_title"),
      description: runError,
      action: (
        <Button variant="secondary" onClick={() => setActiveTab("project")}>
          {t("workspace.run_tab.action_check_preflight")}
        </Button>
      ),
    });
  }

  // 3. Docx locked attention banner
  if (docxStatus.status === "locked" && !dismissedDocx) {
    bannerList.push({
      id: "docx_locked",
      variant: "attention",
      title: t("workspace.run_tab.attention_docx_title"),
      description: t("workspace.run_tab.attention_docx_desc"),
      action: (
        <Button
          variant="secondary"
          onClick={() => void retryDocxRender()}
        >
          {t("workspace.run_tab.action_retry_docx")}
        </Button>
      ),
      onClose: () => setDismissedDocx(true),
    });
  }

  // 3. Chapter needs attention banners
  const attentionChapters = chapters.filter((c) => c.status === "needs_attention");
  for (const ch of attentionChapters) {
    if (!dismissedChapters[ch.id]) {
      bannerList.push({
        id: `chap_attn_${ch.id}`,
        variant: "attention",
        title: t("workspace.run_tab.attention_chapter_title"),
        description: t("workspace.run_tab.attention_chapter_desc", {
          ordinal: String(ch.ordinal).padStart(2, "0"),
        }),
        action: (
          <Button
            variant="secondary"
            onClick={() => {
              selectChapter(ch.id);
              setActiveTab("chapters");
            }}
          >
            {t("workspace.run_tab.action_view_chapter")}
          </Button>
        ),
        onClose: () =>
          setDismissedChapters((prev) => ({ ...prev, [ch.id]: true })),
      });
    }
  }

  if (bannerList.length === 0) return null;

  // Max 3 banners, remaining summarized
  const visibleBanners = bannerList.slice(0, 3);
  const remainingCount = bannerList.length - 3;

  return (
    <div style={{ display: "grid", gap: "var(--space-3)", width: "100%" }}>
      {visibleBanners.map((b) => (
        <Banner
          key={b.id}
          variant={b.variant}
          title={b.title}
          description={b.description}
          action={b.action}
          onClose={b.onClose}
        />
      ))}
      {remainingCount > 0 && (
        <div
          style={{
            font: "var(--text-small)",
            color: "var(--color-text-3)",
            textAlign: "right",
            paddingRight: "var(--space-2)",
          }}
        >
          {t("workspace.run_tab.and_more_banners", { count: remainingCount })}
        </div>
      )}
    </div>
  );
};
