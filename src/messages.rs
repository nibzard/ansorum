use log;
use std::convert::TryInto;
use std::time::Instant;
use time::Duration;

use crate::diagnostics::{CommandFailure, Diagnostic, SiteContentSummary};
use errors::Error;
use site::Site;

fn ignored_page_paths(site: &Site) -> Vec<String> {
    let library = site.library.read().unwrap();
    library
        .sections
        .values()
        .flat_map(|s| {
            s.ignored_pages.iter().map(|k| library.pages[k].file.path.display().to_string())
        })
        .collect()
}

fn orphan_page_paths(site: &Site) -> Vec<String> {
    let library = site.library.read().unwrap();
    library.get_all_orphan_pages().iter().map(|page| page.path.clone()).collect()
}

pub fn collect_site_content_summary(site: &Site) -> SiteContentSummary {
    let library = site.library.read().unwrap();
    SiteContentSummary {
        pages: library.pages.len(),
        orphan_pages: library.get_all_orphan_pages().len(),
        sections: library.sections.len().saturating_sub(1),
        answer_records: site.answers.len(),
    }
}

pub fn collect_ignored_page_diagnostics(site: &Site) -> Vec<Diagnostic> {
    ignored_page_paths(site)
        .into_iter()
        .map(|path| {
            Diagnostic::warn(
                "ignored_page",
                "Page ignored because a sorted section is missing date or weight metadata",
            )
            .with_path(path)
            .with_phase("validate")
            .with_suggestion(
                "Add date/weight metadata or remove sorting requirements for the section",
            )
        })
        .collect()
}

pub fn collect_orphan_page_diagnostics(site: &Site) -> Vec<Diagnostic> {
    orphan_page_paths(site)
        .into_iter()
        .map(|path| {
            Diagnostic::warn("orphan_page", "Page is not linked from any section")
                .with_path(path)
                .with_phase("validate")
                .with_suggestion(
                    "Link the page from a section or mark it intentionally unreachable",
                )
        })
        .collect()
}

/// Display in the console the number of pages/sections in the site
pub fn notify_site_size(site: &Site) {
    let summary = collect_site_content_summary(site);
    log::info!(
        "-> Creating {} pages ({} orphan) and {} sections",
        summary.pages,
        summary.orphan_pages,
        summary.sections,
    );
    log::info!("-> Normalized {} answer record(s)", summary.answer_records);
}

/// Display a warning in the console if there are ignored pages in the site
pub fn warn_about_ignored_pages(site: &Site) {
    let ignored_pages = ignored_page_paths(site);

    if !ignored_pages.is_empty() {
        log::warn!(
            "{} page(s) ignored (missing date or weight in a sorted section):",
            ignored_pages.len()
        );
        for path in ignored_pages {
            log::warn!("- {}", path);
        }
    }
}

pub fn print_build_success(summary: &SiteContentSummary, diagnostics: &[Diagnostic]) {
    log::info!(
        "-> Creating {} pages ({} orphan) and {} sections",
        summary.pages,
        summary.orphan_pages,
        summary.sections,
    );
    log::info!("-> Normalized {} answer record(s)", summary.answer_records);

    for diagnostic in diagnostics {
        print_diagnostic(diagnostic);
    }
}

pub fn print_check_success(summary: &SiteContentSummary, diagnostics: &[Diagnostic]) {
    log::info!(
        "-> Site content: {} pages ({} orphan), {} sections",
        summary.pages,
        summary.orphan_pages,
        summary.sections,
    );
    log::info!("-> Normalized {} answer record(s)", summary.answer_records);

    for diagnostic in diagnostics {
        print_diagnostic(diagnostic);
    }
}

/// Print the time elapsed rounded to 1 decimal
pub fn report_elapsed_time(instant: Instant) {
    let duration: Duration = instant.elapsed().try_into().unwrap();
    let duration_ms = duration.whole_milliseconds() as f64;

    if duration_ms < 1000.0 {
        log::info!("Done in {duration_ms}ms.\n");
    } else {
        let duration_sec = duration_ms / 1000.0;
        log::info!("Done in {:.1}s.\n", ((duration_sec * 10.0).round() / 10.0));
    }
}

/// Display an error message and the actual error(s)
pub fn unravel_errors(message: &str, error: &Error) {
    if !message.is_empty() {
        log::error!("{message}");
    }
    log::error!("{error}");
    let mut cause = error.source();
    while let Some(e) = cause {
        log::error!("Reason: {e}");
        cause = e.source();
    }
}

pub fn print_command_failure(prefix: &str, failure: &CommandFailure) {
    if !prefix.is_empty() {
        log::error!("{prefix}");
    }
    for diagnostic in &failure.diagnostics {
        print_diagnostic(diagnostic);
        if let Some(causes) = diagnostic.caused_by.as_ref() {
            for cause in causes {
                log::error!("Reason: {cause}");
            }
        }
    }
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    let mut parts = vec![format!("{} [{}]", diagnostic.severity.label(), diagnostic.code)];
    if let Some(path) = diagnostic.path.as_deref() {
        if let Some(line) = diagnostic.line {
            if let Some(column) = diagnostic.column {
                parts.push(format!("path={path}:{line}:{column}"));
            } else {
                parts.push(format!("path={path}:{line}"));
            }
        } else {
            parts.push(format!("path={path}"));
        }
    }
    if let Some(answer_id) = diagnostic.answer_id.as_deref() {
        parts.push(format!("answer={answer_id}"));
    }
    if let Some(phase) = diagnostic.phase.as_deref() {
        parts.push(format!("phase={phase}"));
    }

    match diagnostic.severity {
        crate::diagnostics::DiagnosticSeverity::Error => {
            log::error!("{}: {}", parts.join(" "), diagnostic.message)
        }
        crate::diagnostics::DiagnosticSeverity::Warn => {
            log::warn!("{}: {}", parts.join(" "), diagnostic.message)
        }
        crate::diagnostics::DiagnosticSeverity::Info => {
            log::info!("{}: {}", parts.join(" "), diagnostic.message)
        }
    }

    if let Some(suggestion) = diagnostic.suggestion.as_deref() {
        log::info!("suggestion: {suggestion}");
    }
}
