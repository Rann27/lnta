-- Migration 001: Initial LNTA schema (PRD §10.4)

CREATE TABLE IF NOT EXISTS chapter (
    id TEXT PRIMARY KEY NOT NULL,
    ordinal INTEGER NOT NULL,
    file_name TEXT NOT NULL,
    prefix TEXT NOT NULL,
    title_src TEXT NOT NULL,
    title_tgt TEXT,
    src_hash TEXT NOT NULL,
    line_count INTEGER NOT NULL,
    char_count INTEGER NOT NULL,
    status TEXT NOT NULL,
    frozen INTEGER NOT NULL DEFAULT 0,
    parts_planned INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS segment (
    id TEXT PRIMARY KEY NOT NULL,
    chapter_id TEXT NOT NULL REFERENCES chapter(id) ON DELETE CASCADE,
    idx INTEGER NOT NULL,
    kind TEXT NOT NULL,
    src_text TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    image_file TEXT,
    part_index INTEGER
);

CREATE TABLE IF NOT EXISTS chunk_plan (
    id TEXT PRIMARY KEY NOT NULL,
    chapter_id TEXT NOT NULL REFERENCES chapter(id) ON DELETE CASCADE,
    part_index INTEGER NOT NULL,
    seg_from INTEGER NOT NULL,
    seg_to INTEGER NOT NULL,
    line_from INTEGER NOT NULL,
    line_to INTEGER NOT NULL,
    est_in INTEGER NOT NULL,
    est_out INTEGER NOT NULL,
    status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS run (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    settings_snapshot TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS call (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT REFERENCES run(id),
    role TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    chapter_id TEXT REFERENCES chapter(id),
    part_index INTEGER,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    finish_reason TEXT,
    in_tokens INTEGER NOT NULL DEFAULT 0,
    cached_in INTEGER NOT NULL DEFAULT 0,
    cache_write INTEGER NOT NULL DEFAULT 0,
    out_tokens INTEGER NOT NULL DEFAULT 0,
    reasoning_tokens INTEGER NOT NULL DEFAULT 0,
    cost REAL,
    latency_ms INTEGER NOT NULL DEFAULT 0,
    contract_version TEXT,
    manifest_json TEXT,
    error TEXT,
    raw_response TEXT
);

CREATE TABLE IF NOT EXISTS translation (
    id TEXT PRIMARY KEY NOT NULL,
    chapter_id TEXT NOT NULL REFERENCES chapter(id) ON DELETE CASCADE,
    seg_from INTEGER NOT NULL,
    seg_to INTEGER NOT NULL,
    text TEXT NOT NULL,
    origin TEXT NOT NULL,
    call_id TEXT REFERENCES call(id),
    version INTEGER NOT NULL DEFAULT 1,
    is_current INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS glossary_entry (
    id TEXT PRIMARY KEY NOT NULL,
    source TEXT NOT NULL,
    translation TEXT NOT NULL,
    note TEXT,
    tier INTEGER NOT NULL DEFAULT 1,
    origin TEXT NOT NULL DEFAULT 'model',
    first_chapter_id TEXT REFERENCES chapter(id),
    occurrences INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active',
    file_line INTEGER
);

CREATE TABLE IF NOT EXISTS finding (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT REFERENCES run(id),
    check_code TEXT NOT NULL,
    chapter_id TEXT REFERENCES chapter(id),
    segment_id TEXT REFERENCES segment(id),
    severity TEXT NOT NULL,
    detail_json TEXT NOT NULL,
    status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS fix (
    id TEXT PRIMARY KEY NOT NULL,
    finding_id TEXT REFERENCES finding(id),
    translation_id_before TEXT REFERENCES translation(id),
    translation_id_after TEXT REFERENCES translation(id),
    origin TEXT NOT NULL,
    call_id TEXT REFERENCES call(id),
    applied_at TEXT NOT NULL,
    reverted_at TEXT
);

CREATE TABLE IF NOT EXISTS event (
    id TEXT PRIMARY KEY NOT NULL,
    ts TEXT NOT NULL,
    level TEXT NOT NULL,
    code TEXT NOT NULL,
    message_key TEXT NOT NULL,
    params_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chapter_prefix ON chapter(prefix);
CREATE INDEX IF NOT EXISTS idx_segment_chapter ON segment(chapter_id, idx);
CREATE INDEX IF NOT EXISTS idx_translation_lookup ON translation(chapter_id, seg_from, seg_to, is_current);
CREATE INDEX IF NOT EXISTS idx_chunk_plan_chapter ON chunk_plan(chapter_id, part_index);
CREATE INDEX IF NOT EXISTS idx_call_run ON call(run_id);
CREATE INDEX IF NOT EXISTS idx_glossary_source ON glossary_entry(source);
CREATE INDEX IF NOT EXISTS idx_finding_run ON finding(run_id);
CREATE INDEX IF NOT EXISTS idx_fix_finding ON fix(finding_id);
CREATE INDEX IF NOT EXISTS idx_event_ts ON event(ts);
