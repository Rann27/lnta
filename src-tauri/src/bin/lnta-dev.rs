//! Developer CLI for LNTA core per PRD §10.5, AGENTS.md §7, and task T19.
//!
//! Subcommands:
//! - `preflight <folder> [--skip-model-check]`
//! - `plan <folder> --max-output <n>`
//! - `render <folder> --translations <file.json>`

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Deserialize;

use lnta_lib::docx::default_template::generate_default_template;
use lnta_lib::docx::write::render_and_write_docx;
use lnta_lib::docx::{RenderChapter, RenderItem};
use lnta_lib::plan::estimate::{EstimationWeights, ModelLimits};
use lnta_lib::plan::split::plan_chapter_chunks;
use lnta_lib::project::discover::discover_chapters;
use lnta_lib::project::preflight::{run_preflight, PreflightEnvironment, PreflightStatus};
use lnta_lib::raw::classify::IllustrationSet;
use lnta_lib::raw::encoding::read_raw_file;
use lnta_lib::raw::segment::{parse_chapter, ParseOptions};

#[derive(Debug, Deserialize)]
struct TranslationInput {
    #[serde(default)]
    chapters: Vec<ChapterInput>,
}

#[derive(Debug, Deserialize)]
struct ChapterInput {
    pub title: Option<String>,
    #[serde(default)]
    pub is_image_page: bool,
    #[serde(default)]
    pub paragraphs: Vec<String>,
}

fn print_usage() {
    println!("LNTA Developer CLI");
    println!("Usage:");
    println!("  lnta-dev preflight <folder> [--skip-model-check]");
    println!("  lnta-dev plan <folder> --max-output <n>");
    println!("  lnta-dev render <folder> --translations <file.json>");
}

fn determine_profile(folder: &Path) -> &'static str {
    let en_inst = folder.join("instruction-en.md");
    if en_inst.exists() {
        "ja-en"
    } else {
        "ja-id"
    }
}

fn cmd_preflight(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing <folder> argument for preflight".to_string());
    }

    let folder = Path::new(&args[0]);
    let skip_model_check = args.iter().any(|arg| arg == "--skip-model-check");

    let profile = determine_profile(folder);
    let env = PreflightEnvironment {
        skip_model_check,
        ..Default::default()
    };

    let report = run_preflight(folder, profile, &env);

    println!(
        "Preflight report for {} (profile: {}):",
        folder.display(),
        profile
    );
    for item in &report.items {
        let status_tag = match item.status {
            PreflightStatus::Pass => "[PASS]   ",
            PreflightStatus::Info => "[INFO]   ",
            PreflightStatus::Warning => "[WARNING]",
            PreflightStatus::Blocker => "[BLOCKER]",
        };
        println!("  {} {} - {}", status_tag, item.id, item.message_code);
    }

    println!();
    println!(
        "Summary: {} passed, {} info, {} warnings, {} blockers.",
        report.passes().len(),
        report.infos().len(),
        report.warnings().len(),
        report.blockers().len()
    );
    println!("Can run: {}", report.can_run);

    if !report.can_run {
        return Err("Preflight has blockers".to_string());
    }

    Ok(())
}

fn cmd_plan(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing <folder> argument for plan".to_string());
    }

    let folder = Path::new(&args[0]);
    let mut max_output: u32 = 4096;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--max-output" && i + 1 < args.len() {
            max_output = args[i + 1]
                .parse::<u32>()
                .map_err(|e| format!("Invalid --max-output integer: {e}"))?;
            i += 1;
        }
        i += 1;
    }

    let chapters =
        discover_chapters(folder).map_err(|e| format!("Failed to discover chapters: {e}"))?;

    let illustrations_dir = folder.join("illustrations");
    let illustrations = IllustrationSet::from_dir(&illustrations_dir);
    let parse_opts = ParseOptions::default();

    let limits = ModelLimits {
        max_output_tokens: max_output,
        context_window: 128_000,
    };
    let weights = EstimationWeights::default();

    println!(
        "Planning {} chapters with max_output={}...",
        chapters.len(),
        max_output
    );

    for ch in chapters {
        let content = read_raw_file(&ch.path)
            .map_err(|e| format!("Failed to read {}: {e}", ch.path.display()))?;

        let parsed = parse_chapter(&ch.file_name, &content, &illustrations, &parse_opts);
        let total_lines = content.lines().count();

        let plan = plan_chapter_chunks(
            &ch.file_name,
            &parsed.segments,
            total_lines,
            1500,
            &limits,
            &weights,
        );

        println!(
            "Chapter: {} -> {} parts (total lines: {})",
            ch.file_name,
            plan.parts.len(),
            total_lines
        );
        for part in &plan.parts {
            println!(
                "  Part {}: lines {}-{}, segments {}-{}, est_tokens: in={} out={}",
                part.part_index,
                part.line_start,
                part.line_end,
                part.seg_from,
                part.seg_to,
                part.estimated_input_tokens,
                part.estimated_output_tokens
            );
        }
    }

    Ok(())
}

fn cmd_render(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing <folder> argument for render".to_string());
    }

    let folder = Path::new(&args[0]);
    let mut translations_file: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--translations" && i + 1 < args.len() {
            translations_file = Some(PathBuf::from(&args[i + 1]));
            i += 1;
        }
        i += 1;
    }

    let trans_path = translations_file
        .ok_or_else(|| "Missing required --translations <file.json> flag".to_string())?;

    let trans_json = std::fs::read_to_string(&trans_path).map_err(|e| {
        format!(
            "Failed to read translations file {}: {e}",
            trans_path.display()
        )
    })?;

    let input: TranslationInput = serde_json::from_str(&trans_json)
        .or_else(|_| {
            let chs: Vec<ChapterInput> = serde_json::from_str(&trans_json)?;
            Ok(TranslationInput { chapters: chs })
        })
        .map_err(|e: serde_json::Error| format!("Failed to parse translations JSON: {e}"))?;

    let mut render_chapters = Vec::new();
    for ch in input.chapters {
        render_chapters.push(RenderChapter {
            title: ch.title,
            is_image_page: ch.is_image_page,
            items: ch.paragraphs.into_iter().map(RenderItem::Text).collect(),
        });
    }

    // Load template or generate default
    let template_path = folder.join("Translations.docx");
    let template_bytes = if template_path.exists() {
        std::fs::read(&template_path)
            .map_err(|e| format!("Failed to read template {}: {e}", template_path.display()))?
    } else {
        let pkg = generate_default_template("Heading1");
        pkg.to_bytes()
            .map_err(|e| format!("Failed to generate default template: {e}"))?
    };

    let backup_dir = folder.join(".lnta").join("backups");
    let _ = std::fs::create_dir_all(&backup_dir);

    render_and_write_docx(
        &template_bytes,
        &render_chapters,
        &template_path,
        Some(&backup_dir),
    )
    .map_err(|e| format!("Render failed: {e}"))?;

    println!(
        "Successfully rendered {} chapters to {}",
        render_chapters.len(),
        template_path.display()
    );

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return ExitCode::FAILURE;
    }

    let command = &args[0];
    let cmd_args = &args[1..];

    let result = match command.as_str() {
        "preflight" => cmd_preflight(cmd_args),
        "plan" => cmd_plan(cmd_args),
        "render" => cmd_render(cmd_args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("Unknown command: {other}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
