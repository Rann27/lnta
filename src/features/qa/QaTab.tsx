import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { StateBlock } from "../../components/ui/StateBlock";
import { Button } from "../../components/ui/Button";
import { FixDiff } from "../../components/ui/FixDiff";
import { Checkbox } from "../../components/ui/Checkbox";
import { useQaStore } from "./store";
import { useWorkspaceStore } from "../workspace/store";
import styles from "./QaTab.module.css";

export const QaTab: React.FC = () => {
  const { t } = useTranslation();
  const { projectPath, selectChapter, setActiveTab } = useWorkspaceStore();
  const {
    findingsSummary,
    fixLog,
    selectedGroupCode,
    activeSubTab,
    selectedFixIds,
    loading,
    loadQaData,
    setSelectedGroupCode,
    setActiveSubTab,
    toggleFixSelection,
    undoFix,
    undoBatch,
  } = useQaStore();

  useEffect(() => {
    if (projectPath) {
      loadQaData(projectPath);
    }
  }, [projectPath, loadQaData]);

  const isEmpty =
    (!findingsSummary || findingsSummary.total_findings === 0) &&
    fixLog.length === 0;

  if (isEmpty) {
    return (
      <div className={styles.container}>
        <StateBlock
          title={t("qa.empty.title")}
          description={t("qa.empty.description")}
          action={
            <Button
              variant="primary"
              onClick={() => {
                if (projectPath) loadQaData(projectPath);
              }}
              disabled={loading}
            >
              {t("qa.empty.action")}
            </Button>
          }
        />
      </div>
    );
  }

  const allFindings = findingsSummary?.findings ?? [];
  const displayedFindings = selectedGroupCode
    ? allFindings.filter((f) => f.check_code === selectedGroupCode)
    : allFindings;

  const handleJumpToSegment = (chapterId?: string | null) => {
    if (chapterId) {
      selectChapter(chapterId);
      setActiveTab("chapters");
    }
  };

  return (
    <div className={styles.container}>
      {/* 1. Header with stats */}
      <header className={styles.header}>
        <div className={styles.titleArea}>
          <h2 className={styles.title}>{t("qa.title")}</h2>
          <p className={styles.description}>{t("qa.description")}</p>
        </div>

        <div className={styles.statsBar}>
          <div className={styles.statItem}>
            <span>{t("qa.summary.total", { count: findingsSummary?.total_findings ?? 0 })}</span>
          </div>
          <div className={styles.statItem}>
            <span className={styles.statValue}>
              {t("qa.summary.remaining", { count: findingsSummary?.remaining_findings ?? 0 })}
            </span>
          </div>
          <div className={styles.statItem}>
            <span>
              {t("qa.summary.fixed", { count: fixLog.filter((f) => !f.reverted_at).length })}
            </span>
          </div>
        </div>
      </header>

      {/* 2. Sub-tab toggle bar */}
      <nav className={styles.navBar} aria-label={t("qa.title")}>
        <div className={styles.subTabs} role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={activeSubTab === "findings"}
            className={[
              styles.subTabBtn,
              activeSubTab === "findings" ? styles.subTabBtnActive : "",
            ].join(" ")}
            onClick={() => setActiveSubTab("findings")}
          >
            {t("qa.tabs.findings", { count: findingsSummary?.remaining_findings ?? 0 })}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeSubTab === "fix_log"}
            className={[
              styles.subTabBtn,
              activeSubTab === "fix_log" ? styles.subTabBtnActive : "",
            ].join(" ")}
            onClick={() => setActiveSubTab("fix_log")}
          >
            {t("qa.tabs.fixLog", { count: fixLog.length })}
          </button>
        </div>
      </nav>

      {/* 3. Findings Sub-tab */}
      {activeSubTab === "findings" && (
        <section className={styles.contentArea} aria-label={t("qa.findingsList.title")}>
          {/* Check Groups Filter Chips */}
          <div className={styles.groupsBar} role="toolbar" aria-label={t("qa.groups.title")}>
            <button
              type="button"
              className={[
                styles.groupChip,
                selectedGroupCode === null ? styles.groupChipActive : "",
              ].join(" ")}
              onClick={() => setSelectedGroupCode(null)}
            >
              <span>{t("qa.groups.all")}</span>
              <span className={styles.groupCount}>{allFindings.length}</span>
            </button>
            {findingsSummary?.groups.map((group) => (
              <button
                key={group.check_code}
                type="button"
                className={[
                  styles.groupChip,
                  selectedGroupCode === group.check_code ? styles.groupChipActive : "",
                ].join(" ")}
                onClick={() => setSelectedGroupCode(group.check_code)}
              >
                <span>{group.check_code}</span>
                <span className={styles.groupCount}>{group.count}</span>
              </button>
            ))}
          </div>

          {/* Findings List */}
          <div className={styles.findingsList}>
            {displayedFindings.length === 0 ? (
              <StateBlock
                title={t("qa.findingsList.empty")}
                description={t("qa.groups.all")}
              />
            ) : (
              displayedFindings.map((finding) => {
                const locationLabel =
                  finding.chapter_ordinal !== null && finding.chapter_ordinal !== undefined
                    ? t("qa.findingsList.location", {
                        chapter: finding.chapter_ordinal,
                        segment: finding.seg_idx ?? "?",
                      })
                    : t("qa.findingsList.volumeWide");

                return (
                  <article key={finding.id} className={styles.findingCard}>
                    <div className={styles.findingHeader}>
                      <div className={styles.findingMeta}>
                        <span className={styles.codeBadge}>{finding.check_code}</span>
                        <span className={styles.locationLabel}>{locationLabel}</span>
                        <span className={styles.codeBadge}>{finding.status}</span>
                      </div>

                      {finding.chapter_id && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleJumpToSegment(finding.chapter_id)}
                        >
                          {t("qa.findingsList.jumpToSegment")}
                        </Button>
                      )}
                    </div>

                    <div className={styles.findingBody}>
                      {finding.message && (
                        <p className={styles.findingMessage}>{finding.message}</p>
                      )}
                      {finding.matched_text && (
                        <code className={styles.matchedText}>{finding.matched_text}</code>
                      )}
                    </div>
                  </article>
                );
              })
            )}
          </div>
        </section>
      )}

      {/* 4. Fix Log Sub-tab */}
      {activeSubTab === "fix_log" && (
        <section className={styles.contentArea} aria-label={t("qa.fixLog.title")}>
          {fixLog.length === 0 ? (
            <StateBlock
              title={t("qa.fixLog.empty")}
              description={t("qa.empty.description")}
            />
          ) : (
            <>
              {selectedFixIds.length > 0 && (
                <div className={styles.actionBar}>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => {
                      if (projectPath) undoBatch(projectPath, selectedFixIds);
                    }}
                  >
                    {t("qa.action.undoBatch")} ({selectedFixIds.length})
                  </Button>
                </div>
              )}

              <div className={styles.fixList}>
                {fixLog.map((fix) => {
                  const locationLabel =
                    fix.chapter_ordinal !== null && fix.chapter_ordinal !== undefined
                      ? t("qa.findingsList.location", {
                          chapter: fix.chapter_ordinal,
                          segment: fix.seg_idx ?? "?",
                        })
                      : undefined;

                  const isSelected = selectedFixIds.includes(fix.id);

                  return (
                    <div key={fix.id} className={styles.fixRow}>
                      {!fix.reverted_at && (
                        <div className={styles.fixCheckbox}>
                          <Checkbox
                            checked={isSelected}
                            onChange={() => toggleFixSelection(fix.id)}
                            aria-label={t("qa.action.undoFix")}
                          />
                        </div>
                      )}
                      <FixDiff
                        checkCode={fix.check_code ?? undefined}
                        origin={fix.origin}
                        appliedAt={fix.applied_at}
                        revertedAt={fix.reverted_at}
                        textBefore={fix.text_before}
                        textAfter={fix.text_after}
                        locationLabel={locationLabel}
                        onUndo={() => {
                          if (projectPath) undoFix(projectPath, fix.id);
                        }}
                      />
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </section>
      )}
    </div>
  );
};
