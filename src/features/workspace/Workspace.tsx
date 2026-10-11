import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { isTauri } from "@tauri-apps/api/core";
import { RunBar } from "./RunBar";
import { ChapterRail } from "./Rail";
import { StatusBar } from "./StatusBar";
import { LogDrawer } from "./LogDrawer";
import { RunTab } from "./RunTab";
import { ChapterTab } from "../chapters";
import { GlossaryTab } from "../glossary";
import { QaTab } from "../qa";
import { UsageTab } from "../usage";
import { ProjectTab } from "../project";
import { useWorkspaceStore, type WorkspaceTab } from "./store";
import { useProjectsStore } from "../projects/store";
import styles from "./Workspace.module.css";

export const Workspace: React.FC = () => {
  const { t } = useTranslation();
  const {
    projectPath,
    activeTab,
    qaFindingsCount,
    logDrawerOpen,
    setActiveTab,
    initWorkspace,
  } = useWorkspaceStore();
  const { activeProject } = useProjectsStore();
  const navigate = useNavigate();

  const [windowWidth, setWindowWidth] = useState<number>(() =>
    typeof window !== "undefined" ? window.innerWidth : 1280
  );

  const railRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const logRef = useRef<HTMLDivElement>(null);

  // Layout width classes: compact (<1280), regular (1280-1599), wide (>=1600)
  useEffect(() => {
    const handleResize = () => {
      setWindowWidth(window.innerWidth);
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  const isWide = windowWidth >= 1600;
  const isCompact = windowWidth < 1280;
  const layoutClass = isWide
    ? styles.layoutWide
    : isCompact
    ? styles.layoutCompact
    : styles.layoutRegular;

  // Initialize workspace if not initialized yet
  // The browser preview has no folder dialog, so it falls back to its synthetic volume.
  useEffect(() => {
    const targetPath =
      activeProject?.path || projectPath || (isTauri() ? "" : "D:/LN/Synthetic-Volume-1");
    if (!targetPath) {
      navigate("/");
      return;
    }
    if (!projectPath || projectPath !== targetPath) {
      initWorkspace(targetPath);
    }
  }, [activeProject, projectPath, initWorkspace, navigate]);

  // Tab navigation shortcuts (Ctrl+1 to Ctrl+6)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.shiftKey && !e.altKey) {
        if (e.key === "1") {
          e.preventDefault();
          setActiveTab("run");
        } else if (e.key === "2") {
          e.preventDefault();
          setActiveTab("chapters");
        } else if (e.key === "3") {
          e.preventDefault();
          setActiveTab("glossary");
        } else if (e.key === "4") {
          e.preventDefault();
          setActiveTab("qa");
        } else if (e.key === "5") {
          e.preventDefault();
          setActiveTab("usage");
        } else if (e.key === "6") {
          e.preventDefault();
          setActiveTab("project");
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setActiveTab]);

  // F6 Pane cycling: rail -> content -> log -> rail
  useEffect(() => {
    const handleF6 = (e: KeyboardEvent) => {
      if (e.key === "F6") {
        e.preventDefault();
        const panes: (HTMLElement | null)[] = [
          railRef.current?.querySelector("aside") ?? railRef.current,
          contentRef.current,
          (isWide || logDrawerOpen) ? (logRef.current?.querySelector("section") ?? logRef.current) : null,
        ].filter(Boolean);

        if (panes.length === 0) return;

        const currentActive = document.activeElement;
        let currentIndex = panes.findIndex((p) => p === currentActive || p?.contains(currentActive));

        if (e.shiftKey) {
          currentIndex = currentIndex <= 0 ? panes.length - 1 : currentIndex - 1;
        } else {
          currentIndex = currentIndex >= panes.length - 1 ? 0 : currentIndex + 1;
        }

        const targetPane = panes[currentIndex];
        if (targetPane) {
          targetPane.focus();
        }
      }
    };

    window.addEventListener("keydown", handleF6);
    return () => window.removeEventListener("keydown", handleF6);
  }, [isWide, logDrawerOpen]);

  const tabs: { id: WorkspaceTab; label: string; badge?: number }[] = [
    { id: "run", label: t("workspace.tabs.run") },
    { id: "chapters", label: t("workspace.tabs.chapters") },
    { id: "glossary", label: t("workspace.tabs.glossary") },
    { id: "qa", label: t("workspace.tabs.qa"), badge: qaFindingsCount && qaFindingsCount > 0 ? qaFindingsCount : undefined },
    { id: "usage", label: t("workspace.tabs.usage") },
    { id: "project", label: t("workspace.tabs.project") },
  ];

  return (
    <div className={[styles.workspace, layoutClass].join(" ")}>
      {/* 1. Run Bar at top */}
      <RunBar />

      {/* 2. Body Grid */}
      <div className={styles.body}>
        {/* Left: Chapter Rail */}
        <div ref={railRef}>
          <ChapterRail />
        </div>

        {/* Center: Main Content */}
        <main
          ref={contentRef}
          className={styles.mainContent}
          tabIndex={-1}
          aria-label={t("workspace.panes.content")}
        >
          {/* Workspace Tabs */}
          <nav
            className={styles.tabs}
            role="tablist"
            aria-label={t("a11y.workspace_tabs")}
          >
            {tabs.map((tab) => {
              const isSelected = activeTab === tab.id;
              return (
                <button
                  key={tab.id}
                  type="button"
                  role="tab"
                  className={[
                    styles.tab,
                    isSelected ? styles.tabActive : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  aria-selected={isSelected}
                  tabIndex={isSelected ? 0 : -1}
                  onClick={() => setActiveTab(tab.id)}
                >
                  <span>{tab.label}</span>
                  {tab.badge !== undefined && (
                    <span className={styles.tabBadge}>{tab.badge}</span>
                  )}
                </button>
              );
            })}
          </nav>

          {/* Active Tab Panel */}
          <div
            className={styles.tabPanel}
            role="tabpanel"
            id={`panel-${activeTab}`}
            tabIndex={0}
          >
            {activeTab === "run" && <RunTab />}
            {activeTab === "chapters" && <ChapterTab />}
            {activeTab === "glossary" && <GlossaryTab />}
            {activeTab === "qa" && <QaTab />}
            {activeTab === "usage" && <UsageTab />}
            {activeTab === "project" && <ProjectTab />}
          </div>

          {/* Bottom Log Drawer (when not in wide layout) */}
          {!isWide && (
            <div ref={logRef}>
              <LogDrawer isPanel={false} />
            </div>
          )}
        </main>

        {/* Right Log Panel (only in wide layout >= 1600px) */}
        {isWide && (
          <div ref={logRef}>
            <LogDrawer isPanel={true} />
          </div>
        )}
      </div>

      {/* 3. Status Bar at bottom */}
      <StatusBar />
    </div>
  );
};
