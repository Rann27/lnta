//! LNTA SQLite store module.

pub mod repo;
pub mod schema;

pub use repo::*;
pub use schema::{migrate, open_in_memory, open_path, setup_connection};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_and_chapter_crud() {
        let conn = open_in_memory().unwrap();
        let repo = ChapterRepo::new(&conn);

        let chapter = Chapter {
            id: "chap-1".to_string(),
            ordinal: 1,
            file_name: "1_prologue.md".to_string(),
            prefix: "1".to_string(),
            title_src: "プロローグ".to_string(),
            title_tgt: None,
            src_hash: "hash123".to_string(),
            line_count: 100,
            char_count: 1500,
            status: "queued".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        repo.insert(&chapter).unwrap();

        let loaded = repo.get_by_id("chap-1").unwrap().unwrap();
        assert_eq!(loaded.prefix, "1");
        assert_eq!(loaded.status, "queued");

        repo.update_status("chap-1", "running").unwrap();
        let updated = repo.get_by_id("chap-1").unwrap().unwrap();
        assert_eq!(updated.status, "running");

        repo.update_title_tgt("chap-1", "Prolog").unwrap();
        let titled = repo.get_by_id("chap-1").unwrap().unwrap();
        assert_eq!(titled.title_tgt.as_deref(), Some("Prolog"));

        let list = repo.list_all().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_segment_batch_and_list() {
        let conn = open_in_memory().unwrap();
        let chap_repo = ChapterRepo::new(&conn);
        let seg_repo = SegmentRepo::new(&conn);

        let chapter = Chapter {
            id: "chap-seg".to_string(),
            ordinal: 1,
            file_name: "2_chapter.md".to_string(),
            prefix: "2".to_string(),
            title_src: "第1話".to_string(),
            title_tgt: None,
            src_hash: "hash456".to_string(),
            line_count: 50,
            char_count: 500,
            status: "queued".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        chap_repo.insert(&chapter).unwrap();

        let segments = vec![
            Segment {
                id: "seg-1".to_string(),
                chapter_id: "chap-seg".to_string(),
                idx: 1,
                kind: "title".to_string(),
                src_text: "第1話".to_string(),
                line_start: 1,
                line_end: 1,
                image_file: None,
                part_index: Some(0),
            },
            Segment {
                id: "seg-2".to_string(),
                chapter_id: "chap-seg".to_string(),
                idx: 2,
                kind: "narration".to_string(),
                src_text: "春の風が吹いた。".to_string(),
                line_start: 3,
                line_end: 3,
                image_file: None,
                part_index: Some(0),
            },
        ];

        seg_repo.insert_batch(&segments).unwrap();

        let loaded = seg_repo.list_by_chapter("chap-seg").unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].idx, 1);
        assert_eq!(loaded[1].kind, "narration");
    }

    #[test]
    fn test_translation_versioning() {
        let conn = open_in_memory().unwrap();
        let chap_repo = ChapterRepo::new(&conn);
        let trans_repo = TranslationRepo::new(&conn);

        let chapter = Chapter {
            id: "chap-trans".to_string(),
            ordinal: 1,
            file_name: "3_chapter.md".to_string(),
            prefix: "3".to_string(),
            title_src: "第2話".to_string(),
            title_tgt: None,
            src_hash: "hash789".to_string(),
            line_count: 50,
            char_count: 500,
            status: "queued".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        chap_repo.insert(&chapter).unwrap();

        // Version 1
        let v1 = trans_repo
            .insert_version("chap-trans", 1, 1, "Terjemahan awal", "model", None)
            .unwrap();
        assert_eq!(v1.version, 1);
        assert!(v1.is_current);

        let current = trans_repo.get_current_by_chapter("chap-trans").unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].text, "Terjemahan awal");

        // Version 2 (e.g. after manual edit or QA fix)
        let v2 = trans_repo
            .insert_version("chap-trans", 1, 1, "Terjemahan diperbaiki", "manual", None)
            .unwrap();
        assert_eq!(v2.version, 2);
        assert!(v2.is_current);

        let current_after = trans_repo.get_current_by_chapter("chap-trans").unwrap();
        assert_eq!(current_after.len(), 1);
        assert_eq!(current_after[0].text, "Terjemahan diperbaiki");

        let history = trans_repo.list_history("chap-trans", 1, 1).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 1);
        assert!(!history[0].is_current);
        assert_eq!(history[1].version, 2);
        assert!(history[1].is_current);
    }

    #[test]
    fn test_run_call_and_chunk_plan() {
        let conn = open_in_memory().unwrap();
        let run_repo = RunRepo::new(&conn);
        let call_repo = CallRepo::new(&conn);
        let plan_repo = ChunkPlanRepo::new(&conn);
        let chap_repo = ChapterRepo::new(&conn);

        let chapter = Chapter {
            id: "chap-run".to_string(),
            ordinal: 1,
            file_name: "4_chapter.md".to_string(),
            prefix: "4".to_string(),
            title_src: "第3話".to_string(),
            title_tgt: None,
            src_hash: "hash_run".to_string(),
            line_count: 50,
            char_count: 500,
            status: "queued".to_string(),
            frozen: false,
            parts_planned: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        chap_repo.insert(&chapter).unwrap();

        let run = Run {
            id: "run-1".to_string(),
            kind: "translate".to_string(),
            status: "running".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            settings_snapshot: "{}".to_string(),
        };
        run_repo.insert(&run).unwrap();

        let plan = ChunkPlan {
            id: "plan-1".to_string(),
            chapter_id: "chap-run".to_string(),
            part_index: 0,
            seg_from: 1,
            seg_to: 10,
            line_from: 1,
            line_to: 20,
            est_in: 500,
            est_out: 600,
            status: "pending".to_string(),
        };
        plan_repo.insert_batch(&[plan]).unwrap();

        let call = Call {
            id: "call-1".to_string(),
            run_id: Some("run-1".to_string()),
            role: "translator".to_string(),
            provider: "anthropic".to_string(),
            model: "claude-3-7-sonnet".to_string(),
            chapter_id: Some("chap-run".to_string()),
            part_index: Some(0),
            kind: "translate".to_string(),
            status: "success".to_string(),
            finish_reason: Some("stop".to_string()),
            in_tokens: 520,
            cached_in: 0,
            cache_write: 0,
            out_tokens: 610,
            reasoning_tokens: 0,
            cost: Some(0.012),
            latency_ms: 1800,
            contract_version: Some("1.0".to_string()),
            manifest_json: None,
            error: None,
            raw_response: None,
        };
        call_repo.insert(&call).unwrap();

        let loaded_call = call_repo.get_by_id("call-1").unwrap().unwrap();
        assert_eq!(loaded_call.provider, "anthropic");
        assert_eq!(loaded_call.in_tokens, 520);

        run_repo.finish_run("run-1", "completed").unwrap();
        let finished_run = run_repo.get_by_id("run-1").unwrap().unwrap();
        assert_eq!(finished_run.status, "completed");
        assert!(finished_run.ended_at.is_some());
    }

    #[test]
    fn test_glossary_repo() {
        let conn = open_in_memory().unwrap();
        let repo = GlossaryRepo::new(&conn);

        let entry = GlossaryEntry {
            id: "glo-1".to_string(),
            source: "健太".to_string(),
            translation: "Kenta".to_string(),
            note: Some("laki-laki".to_string()),
            tier: 1,
            origin: "model".to_string(),
            first_chapter_id: None,
            occurrences: 5,
            status: "active".to_string(),
            file_line: Some(12),
        };
        repo.insert(&entry).unwrap();

        let loaded = repo.get_by_source("健太").unwrap().unwrap();
        assert_eq!(loaded.translation, "Kenta");

        let mut updated = loaded;
        updated.translation = "Kenta (diperbarui)".to_string();
        repo.update(&updated).unwrap();

        let reloaded = repo.get_by_id("glo-1").unwrap().unwrap();
        assert_eq!(reloaded.translation, "Kenta (diperbarui)");

        let all = repo.list_all().unwrap();
        assert_eq!(all.len(), 1);

        repo.delete("glo-1").unwrap();
        assert!(repo.get_by_id("glo-1").unwrap().is_none());
    }

    #[test]
    fn test_finding_and_fix_repo() {
        let conn = open_in_memory().unwrap();
        let finding_repo = FindingRepo::new(&conn);
        let fix_repo = FixRepo::new(&conn);

        let finding = Finding {
            id: "find-1".to_string(),
            run_id: None,
            check_code: "MACRON".to_string(),
            chapter_id: None,
            segment_id: None,
            severity: "warn".to_string(),
            detail_json: "{}".to_string(),
            status: "open".to_string(),
        };
        finding_repo.insert_batch(&[finding]).unwrap();

        finding_repo.update_status("find-1", "fixed").unwrap();

        let fix = Fix {
            id: "fix-1".to_string(),
            finding_id: Some("find-1".to_string()),
            translation_id_before: None,
            translation_id_after: None,
            origin: "auto".to_string(),
            call_id: None,
            applied_at: chrono::Utc::now().to_rfc3339(),
            reverted_at: None,
        };
        fix_repo.insert(&fix).unwrap();

        let loaded_fix = fix_repo.get_by_id("fix-1").unwrap().unwrap();
        assert!(loaded_fix.reverted_at.is_none());

        fix_repo.revert_fix("fix-1").unwrap();
        let reverted = fix_repo.get_by_id("fix-1").unwrap().unwrap();
        assert!(reverted.reverted_at.is_some());
    }

    #[test]
    fn test_event_repo() {
        let conn = open_in_memory().unwrap();
        let event_repo = EventRepo::new(&conn);

        let event = Event {
            id: "ev-1".to_string(),
            ts: chrono::Utc::now().to_rfc3339(),
            level: "info".to_string(),
            code: "RUN_STARTED".to_string(),
            message_key: "events.run_started".to_string(),
            params_json: "{}".to_string(),
        };
        event_repo.insert(&event).unwrap();

        let list = event_repo.list_recent(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].code, "RUN_STARTED");
    }
}
