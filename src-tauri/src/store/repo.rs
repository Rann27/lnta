use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// Model structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Chapter.ts")]
pub struct Chapter {
    pub id: String,
    pub ordinal: i32,
    pub file_name: String,
    pub prefix: String,
    pub title_src: String,
    pub title_tgt: Option<String>,
    pub src_hash: String,
    pub line_count: i32,
    pub char_count: i32,
    pub status: String,
    pub frozen: bool,
    pub parts_planned: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Segment.ts")]
pub struct Segment {
    pub id: String,
    pub chapter_id: String,
    pub idx: i32,
    pub kind: String,
    pub src_text: String,
    pub line_start: i32,
    pub line_end: i32,
    pub image_file: Option<String>,
    pub part_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Translation.ts")]
pub struct Translation {
    pub id: String,
    pub chapter_id: String,
    pub seg_from: i32,
    pub seg_to: i32,
    pub text: String,
    pub origin: String,
    pub call_id: Option<String>,
    pub version: i32,
    pub is_current: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/ChunkPlan.ts")]
pub struct ChunkPlan {
    pub id: String,
    pub chapter_id: String,
    pub part_index: i32,
    pub seg_from: i32,
    pub seg_to: i32,
    pub line_from: i32,
    pub line_to: i32,
    pub est_in: i32,
    pub est_out: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Run.ts")]
pub struct Run {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub settings_snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/Call.ts")]
pub struct Call {
    pub id: String,
    pub run_id: Option<String>,
    pub role: String,
    pub provider: String,
    pub model: String,
    pub chapter_id: Option<String>,
    pub part_index: Option<i32>,
    pub kind: String,
    pub status: String,
    pub finish_reason: Option<String>,
    pub in_tokens: i64,
    pub cached_in: i64,
    pub cache_write: i64,
    pub out_tokens: i64,
    pub reasoning_tokens: i64,
    pub cost: Option<f64>,
    pub latency_ms: i64,
    pub contract_version: Option<String>,
    pub manifest_json: Option<String>,
    pub error: Option<String>,
    pub raw_response: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/lib/bindings/GlossaryEntry.ts")]
pub struct GlossaryEntry {
    pub id: String,
    pub source: String,
    pub translation: String,
    pub note: Option<String>,
    pub tier: i32,
    pub origin: String,
    pub first_chapter_id: Option<String>,
    pub occurrences: i32,
    pub status: String,
    pub file_line: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnedRatioStats {
    pub completed_chapters_count: usize,
    pub total_completed_chars: usize,
    pub total_translator_in_tokens: u64,
    pub total_translator_out_tokens: u64,
    pub total_translator_calls: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub run_id: Option<String>,
    pub check_code: String,
    pub chapter_id: Option<String>,
    pub segment_id: Option<String>,
    pub severity: String,
    pub detail_json: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fix {
    pub id: String,
    pub finding_id: Option<String>,
    pub translation_id_before: Option<String>,
    pub translation_id_after: Option<String>,
    pub origin: String,
    pub call_id: Option<String>,
    pub applied_at: String,
    pub reverted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub ts: String,
    pub level: String,
    pub code: String,
    pub message_key: String,
    pub params_json: String,
}

// ---------------------------------------------------------------------------
// Repositories
// ---------------------------------------------------------------------------

pub struct ChapterRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> ChapterRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert(&self, chapter: &Chapter) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO chapter (
                id, ordinal, file_name, prefix, title_src, title_tgt,
                src_hash, line_count, char_count, status, frozen,
                parts_planned, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                chapter.id,
                chapter.ordinal,
                chapter.file_name,
                chapter.prefix,
                chapter.title_src,
                chapter.title_tgt,
                chapter.src_hash,
                chapter.line_count,
                chapter.char_count,
                chapter.status,
                chapter.frozen as i32,
                chapter.parts_planned,
                chapter.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Chapter>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ordinal, file_name, prefix, title_src, title_tgt,
                    src_hash, line_count, char_count, status, frozen,
                    parts_planned, updated_at
             FROM chapter WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_by_prefix(&self, prefix: &str) -> Result<Option<Chapter>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ordinal, file_name, prefix, title_src, title_tgt,
                    src_hash, line_count, char_count, status, frozen,
                    parts_planned, updated_at
             FROM chapter WHERE prefix = ?1",
        )?;
        let mut rows = stmt.query([prefix])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_all(&self) -> Result<Vec<Chapter>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ordinal, file_name, prefix, title_src, title_tgt,
                    src_hash, line_count, char_count, status, frozen,
                    parts_planned, updated_at
             FROM chapter ORDER BY ordinal ASC",
        )?;
        let rows = stmt.query_map([], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<(), AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE chapter SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;
        Ok(())
    }

    pub fn update_title_tgt(&self, id: &str, title_tgt: &str) -> Result<(), AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE chapter SET title_tgt = ?1, updated_at = ?2 WHERE id = ?3",
            params![title_tgt, now, id],
        )?;
        Ok(())
    }

    pub fn update_parts_planned(&self, id: &str, parts_planned: i32) -> Result<(), AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE chapter SET parts_planned = ?1, updated_at = ?2 WHERE id = ?3",
            params![parts_planned, now, id],
        )?;
        Ok(())
    }

    fn map_row(row: &Row) -> rusqlite::Result<Chapter> {
        let frozen_int: i32 = row.get(10)?;
        Ok(Chapter {
            id: row.get(0)?,
            ordinal: row.get(1)?,
            file_name: row.get(2)?,
            prefix: row.get(3)?,
            title_src: row.get(4)?,
            title_tgt: row.get(5)?,
            src_hash: row.get(6)?,
            line_count: row.get(7)?,
            char_count: row.get(8)?,
            status: row.get(9)?,
            frozen: frozen_int != 0,
            parts_planned: row.get(11)?,
            updated_at: row.get(12)?,
        })
    }
}

pub struct SegmentRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> SegmentRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Replaces every segment of a chapter, used whenever the raw file is (re)parsed so the
    /// review table, QA, and segment editing always see the current source.
    pub fn replace_for_chapter(
        &self,
        chapter_id: &str,
        segments: &[Segment],
    ) -> Result<(), AppError> {
        self.conn.execute(
            "DELETE FROM segment WHERE chapter_id = ?1",
            params![chapter_id],
        )?;
        self.insert_batch(segments)
    }

    pub fn insert_batch(&self, segments: &[Segment]) -> Result<(), AppError> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO segment (
                    id, chapter_id, idx, kind, src_text, line_start,
                    line_end, image_file, part_index
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for s in segments {
                stmt.execute(params![
                    s.id,
                    s.chapter_id,
                    s.idx,
                    s.kind,
                    s.src_text,
                    s.line_start,
                    s.line_end,
                    s.image_file,
                    s.part_index,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_by_chapter(&self, chapter_id: &str) -> Result<Vec<Segment>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, idx, kind, src_text, line_start,
                    line_end, image_file, part_index
             FROM segment WHERE chapter_id = ?1 ORDER BY idx ASC",
        )?;
        let rows = stmt.query_map([chapter_id], |row| {
            Ok(Segment {
                id: row.get(0)?,
                chapter_id: row.get(1)?,
                idx: row.get(2)?,
                kind: row.get(3)?,
                src_text: row.get(4)?,
                line_start: row.get(5)?,
                line_end: row.get(6)?,
                image_file: row.get(7)?,
                part_index: row.get(8)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Segment>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, idx, kind, src_text, line_start,
                    line_end, image_file, part_index
             FROM segment WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Segment {
                id: row.get(0)?,
                chapter_id: row.get(1)?,
                idx: row.get(2)?,
                kind: row.get(3)?,
                src_text: row.get(4)?,
                line_start: row.get(5)?,
                line_end: row.get(6)?,
                image_file: row.get(7)?,
                part_index: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }
}

pub struct TranslationRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> TranslationRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Inserts a new version of translation for a segment range.
    /// Sets previous current translations for this segment range to `is_current = 0`,
    /// and assigns an incremented version number.
    pub fn insert_version(
        &self,
        chapter_id: &str,
        seg_from: i32,
        seg_to: i32,
        text: &str,
        origin: &str,
        call_id: Option<&str>,
    ) -> Result<Translation, AppError> {
        let tx = self.conn.unchecked_transaction()?;

        // Find max existing version
        let prev_version: i32 = tx
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM translation
                 WHERE chapter_id = ?1 AND seg_from = ?2 AND seg_to = ?3",
                params![chapter_id, seg_from, seg_to],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let next_version = prev_version + 1;

        // Invalidate every current version that overlaps this range, not only an identical
        // range: a retranslation may merge segments differently (12-14 vs 12 and 13-14), and
        // a stale overlapping row would render the same text twice in the docx.
        tx.execute(
            "UPDATE translation SET is_current = 0
             WHERE chapter_id = ?1 AND is_current = 1 AND seg_from <= ?3 AND seg_to >= ?2",
            params![chapter_id, seg_from, seg_to],
        )?;

        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        tx.execute(
            "INSERT INTO translation (
                id, chapter_id, seg_from, seg_to, text, origin,
                call_id, version, is_current, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9)",
            params![
                id,
                chapter_id,
                seg_from,
                seg_to,
                text,
                origin,
                call_id,
                next_version,
                created_at,
            ],
        )?;

        tx.commit()?;

        Ok(Translation {
            id,
            chapter_id: chapter_id.to_string(),
            seg_from,
            seg_to,
            text: text.to_string(),
            origin: origin.to_string(),
            call_id: call_id.map(String::from),
            version: next_version,
            is_current: true,
            created_at,
        })
    }

    pub fn get_current_by_chapter(&self, chapter_id: &str) -> Result<Vec<Translation>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, seg_from, seg_to, text, origin,
                    call_id, version, is_current, created_at
             FROM translation
             WHERE chapter_id = ?1 AND is_current = 1
             ORDER BY seg_from ASC",
        )?;
        let rows = stmt.query_map([chapter_id], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn list_history(
        &self,
        chapter_id: &str,
        seg_from: i32,
        seg_to: i32,
    ) -> Result<Vec<Translation>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, seg_from, seg_to, text, origin,
                    call_id, version, is_current, created_at
             FROM translation
             WHERE chapter_id = ?1 AND seg_from = ?2 AND seg_to = ?3
             ORDER BY version ASC",
        )?;
        let rows = stmt.query_map(params![chapter_id, seg_from, seg_to], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Translation>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, seg_from, seg_to, text, origin,
                    call_id, version, is_current, created_at
             FROM translation WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    fn map_row(row: &Row) -> rusqlite::Result<Translation> {
        let is_current_int: i32 = row.get(8)?;
        Ok(Translation {
            id: row.get(0)?,
            chapter_id: row.get(1)?,
            seg_from: row.get(2)?,
            seg_to: row.get(3)?,
            text: row.get(4)?,
            origin: row.get(5)?,
            call_id: row.get(6)?,
            version: row.get(7)?,
            is_current: is_current_int != 0,
            created_at: row.get(9)?,
        })
    }
}

pub struct ChunkPlanRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> ChunkPlanRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Drops the previous plan of a chapter so it can be planned again on retranslate or resume.
    pub fn delete_by_chapter(&self, chapter_id: &str) -> Result<(), AppError> {
        self.conn.execute(
            "DELETE FROM chunk_plan WHERE chapter_id = ?1",
            params![chapter_id],
        )?;
        Ok(())
    }

    pub fn insert_batch(&self, plans: &[ChunkPlan]) -> Result<(), AppError> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO chunk_plan (
                    id, chapter_id, part_index, seg_from, seg_to,
                    line_from, line_to, est_in, est_out, status
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for p in plans {
                stmt.execute(params![
                    p.id,
                    p.chapter_id,
                    p.part_index,
                    p.seg_from,
                    p.seg_to,
                    p.line_from,
                    p.line_to,
                    p.est_in,
                    p.est_out,
                    p.status,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_by_chapter(&self, chapter_id: &str) -> Result<Vec<ChunkPlan>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chapter_id, part_index, seg_from, seg_to,
                    line_from, line_to, est_in, est_out, status
             FROM chunk_plan WHERE chapter_id = ?1 ORDER BY part_index ASC",
        )?;
        let rows = stmt.query_map([chapter_id], |row| {
            Ok(ChunkPlan {
                id: row.get(0)?,
                chapter_id: row.get(1)?,
                part_index: row.get(2)?,
                seg_from: row.get(3)?,
                seg_to: row.get(4)?,
                line_from: row.get(5)?,
                line_to: row.get(6)?,
                est_in: row.get(7)?,
                est_out: row.get(8)?,
                status: row.get(9)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE chunk_plan SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }
}

pub struct RunRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> RunRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert(&self, run: &Run) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO run (
                id, kind, status, started_at, ended_at, settings_snapshot
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                run.id,
                run.kind,
                run.status,
                run.started_at,
                run.ended_at,
                run.settings_snapshot,
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Run>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, status, started_at, ended_at, settings_snapshot
             FROM run WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Run {
                id: row.get(0)?,
                kind: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                settings_snapshot: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE run SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn finish_run(&self, id: &str, status: &str) -> Result<(), AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE run SET status = ?1, ended_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;
        Ok(())
    }

    pub fn get_latest(&self) -> Result<Option<Run>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, status, started_at, ended_at, settings_snapshot
             FROM run ORDER BY started_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Run {
                id: row.get(0)?,
                kind: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                settings_snapshot: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }
}

pub struct CallRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> CallRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert(&self, call: &Call) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO call (
                id, run_id, role, provider, model, chapter_id, part_index,
                kind, status, finish_reason, in_tokens, cached_in,
                cache_write, out_tokens, reasoning_tokens, cost,
                latency_ms, contract_version, manifest_json, error, raw_response
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                call.id,
                call.run_id,
                call.role,
                call.provider,
                call.model,
                call.chapter_id,
                call.part_index,
                call.kind,
                call.status,
                call.finish_reason,
                call.in_tokens,
                call.cached_in,
                call.cache_write,
                call.out_tokens,
                call.reasoning_tokens,
                call.cost,
                call.latency_ms,
                call.contract_version,
                call.manifest_json,
                call.error,
                call.raw_response,
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Call>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, role, provider, model, chapter_id, part_index,
                    kind, status, finish_reason, in_tokens, cached_in,
                    cache_write, out_tokens, reasoning_tokens, cost,
                    latency_ms, contract_version, manifest_json, error, raw_response
             FROM call WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_by_run(&self, run_id: &str) -> Result<Vec<Call>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, role, provider, model, chapter_id, part_index,
                    kind, status, finish_reason, in_tokens, cached_in,
                    cache_write, out_tokens, reasoning_tokens, cost,
                    latency_ms, contract_version, manifest_json, error, raw_response
             FROM call WHERE run_id = ?1 ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([run_id], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn list_by_chapter(&self, chapter_id: &str) -> Result<Vec<Call>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, role, provider, model, chapter_id, part_index,
                    kind, status, finish_reason, in_tokens, cached_in,
                    cache_write, out_tokens, reasoning_tokens, cost,
                    latency_ms, contract_version, manifest_json, error, raw_response
             FROM call WHERE chapter_id = ?1 ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([chapter_id], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn list_all(&self) -> Result<Vec<Call>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, role, provider, model, chapter_id, part_index,
                    kind, status, finish_reason, in_tokens, cached_in,
                    cache_write, out_tokens, reasoning_tokens, cost,
                    latency_ms, contract_version, manifest_json, error, raw_response
             FROM call ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    /// Fetches statistics from completed chapters and translator calls for ratio learning.
    pub fn get_learned_ratio_stats(&self) -> Result<Option<LearnedRatioStats>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                COUNT(DISTINCT ch.id),
                COALESCE(SUM(c.in_tokens), 0),
                COALESCE(SUM(c.out_tokens), 0),
                COUNT(c.id)
             FROM chapter ch
             JOIN call c ON c.chapter_id = ch.id
             WHERE ch.status = 'done'
               AND c.role = 'translator'
               AND (c.status = 'ok' OR c.status = 'completed')",
        )?;
        let row = stmt.query_row([], |r| {
            let ch_count: i64 = r.get(0)?;
            let in_tok: i64 = r.get(1)?;
            let out_tok: i64 = r.get(2)?;
            let calls: i64 = r.get(3)?;
            Ok((ch_count, in_tok, out_tok, calls))
        })?;

        let (ch_count, in_tok, out_tok, calls) = row;
        if ch_count == 0 || calls == 0 || in_tok <= 0 || out_tok <= 0 {
            return Ok(None);
        }

        let mut char_stmt = self.conn.prepare(
            "SELECT COALESCE(SUM(char_count), 0)
             FROM chapter
             WHERE status = 'done'
               AND id IN (
                   SELECT DISTINCT chapter_id FROM call
                   WHERE role = 'translator' AND (status = 'ok' OR status = 'completed')
               )",
        )?;
        let total_chars: i64 = char_stmt.query_row([], |r| r.get(0))?;
        if total_chars <= 0 {
            return Ok(None);
        }

        Ok(Some(LearnedRatioStats {
            completed_chapters_count: ch_count as usize,
            total_completed_chars: total_chars as usize,
            total_translator_in_tokens: in_tok as u64,
            total_translator_out_tokens: out_tok as u64,
            total_translator_calls: calls as usize,
        }))
    }

    fn map_row(row: &Row) -> rusqlite::Result<Call> {
        Ok(Call {
            id: row.get(0)?,
            run_id: row.get(1)?,
            role: row.get(2)?,
            provider: row.get(3)?,
            model: row.get(4)?,
            chapter_id: row.get(5)?,
            part_index: row.get(6)?,
            kind: row.get(7)?,
            status: row.get(8)?,
            finish_reason: row.get(9)?,
            in_tokens: row.get(10)?,
            cached_in: row.get(11)?,
            cache_write: row.get(12)?,
            out_tokens: row.get(13)?,
            reasoning_tokens: row.get(14)?,
            cost: row.get(15)?,
            latency_ms: row.get(16)?,
            contract_version: row.get(17)?,
            manifest_json: row.get(18)?,
            error: row.get(19)?,
            raw_response: row.get(20)?,
        })
    }
}

pub struct GlossaryRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> GlossaryRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert(&self, entry: &GlossaryEntry) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO glossary_entry (
                id, source, translation, note, tier, origin,
                first_chapter_id, occurrences, status, file_line
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.id,
                entry.source,
                entry.translation,
                entry.note,
                entry.tier,
                entry.origin,
                entry.first_chapter_id,
                entry.occurrences,
                entry.status,
                entry.file_line,
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<GlossaryEntry>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, translation, note, tier, origin,
                    first_chapter_id, occurrences, status, file_line
             FROM glossary_entry WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_by_source(&self, source: &str) -> Result<Option<GlossaryEntry>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, translation, note, tier, origin,
                    first_chapter_id, occurrences, status, file_line
             FROM glossary_entry WHERE source = ?1",
        )?;
        let mut rows = stmt.query([source])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_all(&self) -> Result<Vec<GlossaryEntry>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, translation, note, tier, origin,
                    first_chapter_id, occurrences, status, file_line
             FROM glossary_entry ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn update(&self, entry: &GlossaryEntry) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE glossary_entry SET
                translation = ?1, note = ?2, tier = ?3, origin = ?4,
                occurrences = ?5, status = ?6, file_line = ?7
             WHERE id = ?8",
            params![
                entry.translation,
                entry.note,
                entry.tier,
                entry.origin,
                entry.occurrences,
                entry.status,
                entry.file_line,
                entry.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        self.conn
            .execute("DELETE FROM glossary_entry WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn map_row(row: &Row) -> rusqlite::Result<GlossaryEntry> {
        Ok(GlossaryEntry {
            id: row.get(0)?,
            source: row.get(1)?,
            translation: row.get(2)?,
            note: row.get(3)?,
            tier: row.get(4)?,
            origin: row.get(5)?,
            first_chapter_id: row.get(6)?,
            occurrences: row.get(7)?,
            status: row.get(8)?,
            file_line: row.get(9)?,
        })
    }
}

pub struct FindingRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> FindingRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert_batch(&self, findings: &[Finding]) -> Result<(), AppError> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO finding (
                    id, run_id, check_code, chapter_id, segment_id,
                    severity, detail_json, status
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for f in findings {
                stmt.execute(params![
                    f.id,
                    f.run_id,
                    f.check_code,
                    f.chapter_id,
                    f.segment_id,
                    f.severity,
                    f.detail_json,
                    f.status,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_by_run(&self, run_id: &str) -> Result<Vec<Finding>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, check_code, chapter_id, segment_id,
                    severity, detail_json, status
             FROM finding WHERE run_id = ?1 ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([run_id], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn list_by_chapter(&self, chapter_id: &str) -> Result<Vec<Finding>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, check_code, chapter_id, segment_id,
                    severity, detail_json, status
             FROM finding WHERE chapter_id = ?1 ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([chapter_id], Self::map_row)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE finding SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    fn map_row(row: &Row) -> rusqlite::Result<Finding> {
        Ok(Finding {
            id: row.get(0)?,
            run_id: row.get(1)?,
            check_code: row.get(2)?,
            chapter_id: row.get(3)?,
            segment_id: row.get(4)?,
            severity: row.get(5)?,
            detail_json: row.get(6)?,
            status: row.get(7)?,
        })
    }
}

pub struct FixRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> FixRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert(&self, fix: &Fix) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO fix (
                id, finding_id, translation_id_before, translation_id_after,
                origin, call_id, applied_at, reverted_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                fix.id,
                fix.finding_id,
                fix.translation_id_before,
                fix.translation_id_after,
                fix.origin,
                fix.call_id,
                fix.applied_at,
                fix.reverted_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<Fix>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, finding_id, translation_id_before, translation_id_after,
                    origin, call_id, applied_at, reverted_at
             FROM fix WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Fix {
                id: row.get(0)?,
                finding_id: row.get(1)?,
                translation_id_before: row.get(2)?,
                translation_id_after: row.get(3)?,
                origin: row.get(4)?,
                call_id: row.get(5)?,
                applied_at: row.get(6)?,
                reverted_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn revert_fix(&self, id: &str) -> Result<(), AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE fix SET reverted_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn list_by_finding(&self, finding_id: &str) -> Result<Vec<Fix>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, finding_id, translation_id_before, translation_id_after,
                    origin, call_id, applied_at, reverted_at
             FROM fix WHERE finding_id = ?1 ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([finding_id], |row| {
            Ok(Fix {
                id: row.get(0)?,
                finding_id: row.get(1)?,
                translation_id_before: row.get(2)?,
                translation_id_after: row.get(3)?,
                origin: row.get(4)?,
                call_id: row.get(5)?,
                applied_at: row.get(6)?,
                reverted_at: row.get(7)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }
}

pub struct EventRepo<'a> {
    pub conn: &'a Connection,
}

impl<'a> EventRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert(&self, event: &Event) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO event (
                id, ts, level, code, message_key, params_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                event.ts,
                event.level,
                event.code,
                event.message_key,
                event.params_json,
            ],
        )?;
        Ok(())
    }

    pub fn list_recent(&self, limit: i64) -> Result<Vec<Event>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts, level, code, message_key, params_json
             FROM event ORDER BY ts DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |row| {
            Ok(Event {
                id: row.get(0)?,
                ts: row.get(1)?,
                level: row.get(2)?,
                code: row.get(3)?,
                message_key: row.get(4)?,
                params_json: row.get(5)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }
}
