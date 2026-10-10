import React, { useEffect, useRef, useState, useMemo } from "react";
import {
  CaretUp,
  CaretDown,
  Plus,
  ArrowClockwise,
  BookOpenText,
} from "@phosphor-icons/react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/Button";
import { SearchField } from "../../components/ui/SearchField";
import { StateBlock } from "../../components/ui/StateBlock";
import { Dialog } from "../../components/ui/Dialog";
import { TextField } from "../../components/ui/TextField";
import { GlossaryRow } from "./GlossaryRow";
import { useGlossaryStore, type GlossarySortColumn } from "./store";
import { useWorkspaceStore } from "../workspace/store";
import { ipc } from "../../lib/ipc";
import type { GlossaryEntryInput } from "../../lib/bindings";
import styles from "./GlossaryTab.module.css";

export const GlossaryTab: React.FC = () => {
  const { t } = useTranslation();
  const projectPath = useWorkspaceStore((s) => s.projectPath);

  const {
    entries,
    isLoading,
    searchQuery,
    sortColumn,
    sortDirection,
    editingId,
    loadGlossary,
    setSearchQuery,
    setSorting,
    setEditingId,
    addEntry,
  } = useGlossaryStore();

  const [isAddOpen, setIsAddOpen] = useState(false);
  const [newSource, setNewSource] = useState("");
  const [newTranslation, setNewTranslation] = useState("");
  const [newNote, setNewNote] = useState("");

  const tableBodyRef = useRef<HTMLDivElement>(null);

  // Load glossary on mount or projectPath change
  useEffect(() => {
    if (projectPath) {
      void loadGlossary(projectPath);
    }
  }, [projectPath, loadGlossary]);

  // Listen to engine events for real-time reload on disk change
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    ipc
      .onEngineEvent((event) => {
        if (event.event === "glossary:changed") {
          if (projectPath) {
            void loadGlossary(projectPath);
          }
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // ignore listener failure in mock
      });

    return () => {
      unlisten?.();
    };
  }, [projectPath, loadGlossary]);

  // Filter and sort entries
  const filteredAndSorted = useMemo(() => {
    let result = entries;

    // Filter
    const q = searchQuery.trim().toLowerCase();
    if (q) {
      result = result.filter(
        (e) =>
          e.source.toLowerCase().includes(q) ||
          e.translation.toLowerCase().includes(q) ||
          (e.note && e.note.toLowerCase().includes(q))
      );
    }

    // Sort
    return [...result].sort((a, b) => {
      let valA: string | number;
      let valB: string | number;

      switch (sortColumn) {
        case "source":
          valA = a.source;
          valB = b.source;
          break;
        case "translation":
          valA = a.translation;
          valB = b.translation;
          break;
        case "note":
          valA = a.note ?? "";
          valB = b.note ?? "";
          break;
        case "tier":
          valA = a.tier;
          valB = b.tier;
          break;
        case "first_chapter_id":
          valA = a.first_chapter_id ?? "";
          valB = b.first_chapter_id ?? "";
          break;
        case "occurrences":
        default:
          valA = a.occurrences;
          valB = b.occurrences;
          break;
      }

      if (typeof valA === "number" && typeof valB === "number") {
        return sortDirection === "asc" ? valA - valB : valB - valA;
      }
      const strA = String(valA).toLowerCase();
      const strB = String(valB).toLowerCase();
      return sortDirection === "asc"
        ? strA.localeCompare(strB)
        : strB.localeCompare(strA);
    });
  }, [entries, searchQuery, sortColumn, sortDirection]);

  // Virtualizer for smooth scrolling over large sets (e.g. 5,000 entries)
  const isLargeList = filteredAndSorted.length > 200;
  const rowVirtualizer = useVirtualizer({
    count: filteredAndSorted.length,
    getScrollElement: () => tableBodyRef.current,
    estimateSize: () => 36,
    overscan: 10,
    enabled: isLargeList,
  });

  const handleOpenAdd = () => {
    setNewSource("");
    setNewTranslation("");
    setNewNote("");
    setIsAddOpen(true);
  };

  const handleSaveAdd = async () => {
    if (!newSource.trim() || !newTranslation.trim()) return;
    const input: GlossaryEntryInput = {
      source: newSource.trim(),
      translation: newTranslation.trim(),
      note: newNote.trim() || null,
    };
    await addEntry(projectPath, input);
    setIsAddOpen(false);
  };

  const renderSortIndicator = (col: GlossarySortColumn) => {
    if (sortColumn !== col) return null;
    return sortDirection === "asc" ? (
      <CaretUp size={12} aria-hidden="true" />
    ) : (
      <CaretDown size={12} aria-hidden="true" />
    );
  };

  return (
    <div className={styles.container}>
      {/* Top Bar Controls */}
      <header className={styles.topBar}>
        <div className={styles.searchGroup}>
          <SearchField
            value={searchQuery}
            onChange={(val) => setSearchQuery(val)}
            debounceMs={0}
            placeholder={t("glossary.search_placeholder")}
          />
          <span className={styles.countTag}>
            {t("glossary.count", { count: filteredAndSorted.length })}
          </span>
        </div>

        <div className={styles.actionsGroup}>
          <Button
            size="sm"
            variant="secondary"
            icon={<ArrowClockwise size={14} />}
            disabled={isLoading}
            onClick={() => projectPath && void loadGlossary(projectPath)}
          >
            {t("glossary.reload")}
          </Button>

          <Button
            size="sm"
            variant="primary"
            icon={<Plus size={14} />}
            onClick={handleOpenAdd}
          >
            {t("glossary.add_entry")}
          </Button>
        </div>
      </header>

      {/* Table Content */}
      {entries.length === 0 ? (
        <StateBlock
          title={t("glossary.empty_title")}
          description={t("glossary.empty_desc")}
          action={
            <Button variant="primary" icon={<Plus size={14} />} onClick={handleOpenAdd}>
              {t("glossary.add_entry")}
            </Button>
          }
        >
          <BookOpenText size={32} />
        </StateBlock>
      ) : filteredAndSorted.length === 0 ? (
        <StateBlock
          title={t("glossary.empty_search_title")}
          description={t("glossary.empty_search_desc")}
          action={
            <Button variant="secondary" onClick={() => setSearchQuery("")}>
              {t("glossary.clear_search")}
            </Button>
          }
        />
      ) : (
        <div className={styles.tableWrap}>
          {/* Header */}
          <div className={styles.tableHeader} role="row">
            <div>
              <button
                type="button"
                className={styles.thButton}
                onClick={() => setSorting("source")}
                aria-sort={sortColumn === "source" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
              >
                <span>{t("glossary.columns.source")}</span>
                {renderSortIndicator("source")}
              </button>
            </div>
            <div>
              <button
                type="button"
                className={styles.thButton}
                onClick={() => setSorting("translation")}
                aria-sort={sortColumn === "translation" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
              >
                <span>{t("glossary.columns.translation")}</span>
                {renderSortIndicator("translation")}
              </button>
            </div>
            <div>
              <button
                type="button"
                className={styles.thButton}
                onClick={() => setSorting("note")}
                aria-sort={sortColumn === "note" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
              >
                <span>{t("glossary.columns.note")}</span>
                {renderSortIndicator("note")}
              </button>
            </div>
            <div>
              <button
                type="button"
                className={styles.thButton}
                onClick={() => setSorting("tier")}
                aria-sort={sortColumn === "tier" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
              >
                <span>{t("glossary.columns.tier")}</span>
                {renderSortIndicator("tier")}
              </button>
            </div>
            <div>
              <button
                type="button"
                className={styles.thButton}
                onClick={() => setSorting("first_chapter_id")}
                aria-sort={sortColumn === "first_chapter_id" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
              >
                <span>{t("glossary.columns.first_chapter")}</span>
                {renderSortIndicator("first_chapter_id")}
              </button>
            </div>
            <div>
              <button
                type="button"
                className={[styles.thButton, styles.thRight].join(" ")}
                onClick={() => setSorting("occurrences")}
                aria-sort={sortColumn === "occurrences" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
              >
                <span>{t("glossary.columns.occurrences")}</span>
                {renderSortIndicator("occurrences")}
              </button>
            </div>
            <div>{t("glossary.columns.actions")}</div>
          </div>

          {/* Table Body (Virtualized when > 200 entries) */}
          <div ref={tableBodyRef} className={styles.tableBody}>
            {isLargeList ? (
              <div
                className={styles.virtualScrollArea}
                style={{ height: `${rowVirtualizer.getTotalSize()}px` }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const entry = filteredAndSorted[virtualRow.index];
                  return (
                    <div
                      key={entry.id}
                      style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        width: "100%",
                        transform: `translateY(${virtualRow.start}px)`,
                      }}
                    >
                      <GlossaryRow
                        entry={entry}
                        allEntries={entries}
                        projectPath={projectPath}
                        isEditing={editingId === entry.id}
                        onStartEdit={() => setEditingId(entry.id)}
                        onCancelEdit={() => setEditingId(null)}
                      />
                    </div>
                  );
                })}
              </div>
            ) : (
              filteredAndSorted.map((entry) => (
                <GlossaryRow
                  key={entry.id}
                  entry={entry}
                  allEntries={entries}
                  projectPath={projectPath}
                  isEditing={editingId === entry.id}
                  onStartEdit={() => setEditingId(entry.id)}
                  onCancelEdit={() => setEditingId(null)}
                />
              ))
            )}
          </div>
        </div>
      )}

      {/* Add Entry Dialog */}
      <Dialog
        isOpen={isAddOpen}
        onClose={() => setIsAddOpen(false)}
        title={t("glossary.dialog.add_title")}
        actions={
          <>
            <Button variant="secondary" onClick={() => setIsAddOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="primary" onClick={handleSaveAdd}>
              {t("common.save")}
            </Button>
          </>
        }
      >
        <div className={styles.dialogForm}>
          <TextField
            label={t("glossary.dialog.source_label")}
            value={newSource}
            onChange={(e) => setNewSource(e.target.value)}
            placeholder="misal: 健太"
            lang="ja"
          />

          <TextField
            label={t("glossary.dialog.translation_label")}
            value={newTranslation}
            onChange={(e) => setNewTranslation(e.target.value)}
            placeholder="misal: Kenta"
          />

          <TextField
            label={t("glossary.dialog.note_label")}
            value={newNote}
            onChange={(e) => setNewNote(e.target.value)}
            placeholder="misal: Tokoh utama laki-laki"
          />
        </div>
      </Dialog>
    </div>
  );
};
