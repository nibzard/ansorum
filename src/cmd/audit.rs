use std::path::Path;
use std::time::Instant;

use chrono::Utc;
use errors::Result;
use serde_json::json;
use site::Site;
use site::answers::{AuditReport, audit_library};

use crate::cli::{AuditFormat, FailOn};
use crate::diagnostics::{
    self, CommandFailure, CommandReport, CommandSuccess, Diagnostic, DiagnosticSeverity, ReportOutcome,
};
use crate::observability::{self, DispatchMode};

pub fn audit(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    format: AuditFormat,
    fail_on: FailOn,
) -> Result<()> {
    let start = Instant::now();
    let mut site = match Site::new(root_dir, config_file) {
        Ok(site) => site,
        Err(error) => {
            emit_audit_failure(
                root_dir,
                config_file,
                include_drafts,
                format,
                "site_init_failed",
                &error.to_string(),
            );
            print_failure_report(
                root_dir,
                config_file,
                format,
                "site_init_failed",
                "load",
                &error.to_string(),
                start.elapsed(),
            )?;
            return Err(error);
        }
    };
    if include_drafts {
        site.include_drafts();
    }
    if let Err(error) = site.load() {
        emit_audit_failure(
            root_dir,
            config_file,
            include_drafts,
            format,
            "site_load_failed",
            &error.to_string(),
        );
        print_failure_report(
            root_dir,
            config_file,
            format,
            "site_load_failed",
            "load",
            &error.to_string(),
            start.elapsed(),
        )?;
        return Err(error);
    }

    let today = Utc::now().date_naive();
    let library = site.library.read().unwrap();
    let report = audit_library(&library, &site.answers, today);
    drop(library);

    emit_audit_report(root_dir, config_file, include_drafts, format, today, &report);

    let mut diagnostics = audit_report_diagnostics(&report);
    let threshold_exceeded = if fail_on == FailOn::Warn {
        let summary = diagnostics::DiagnosticSummary::from_diagnostics(&diagnostics);
        if fail_on.threshold_exceeded(summary.errors, summary.warnings) {
            diagnostics.push(
                Diagnostic::error(
                    "warn_threshold_exceeded",
                    "Audit completed with warnings, but `--fail-on warn` requires a warning-free run",
                )
                .with_phase("validate")
                .with_suggestion("Resolve warning findings or re-run with `--fail-on error`"),
            );
            true
        } else {
            false
        }
    } else {
        false
    };

    match format {
        AuditFormat::Human => print_human_report(&report),
        AuditFormat::Json | AuditFormat::JsonStream => {
            print_json_report(
                root_dir,
                config_file,
                &report,
                diagnostics,
                start.elapsed(),
                format,
                threshold_exceeded,
            )?
        }
    }

    if report.has_errors() || threshold_exceeded {
        return Err(errors::Error::msg("Audit failed"));
    }

    Ok(())
}

fn print_human_report(report: &AuditReport) {
    println!(
        "Audit summary: {} error(s), {} warning(s), {} info finding(s)",
        report.summary.errors, report.summary.warnings, report.summary.infos
    );

    if report.findings.is_empty() {
        println!("No findings.");
        return;
    }

    for finding in &report.findings {
        let mut parts = vec![format!("{} [{}]", finding.severity.label(), finding.code)];
        if let Some(answer_id) = finding.answer_id.as_deref() {
            parts.push(format!("answer={answer_id}"));
        }
        if let Some(source_path) = finding.source_path.as_deref() {
            parts.push(format!("path={source_path}"));
        }

        println!("{}: {}", parts.join(" "), finding.message);
    }
}

fn audit_report_diagnostics(report: &AuditReport) -> Vec<Diagnostic> {
    report
        .findings
        .iter()
        .map(|finding| {
            let severity = match finding.severity {
                site::answers::AuditSeverity::Error => DiagnosticSeverity::Error,
                site::answers::AuditSeverity::Warn => DiagnosticSeverity::Warn,
                site::answers::AuditSeverity::Info => DiagnosticSeverity::Info,
            };
            let mut diagnostic = Diagnostic {
                code: finding.code.clone(),
                severity,
                message: finding.message.clone(),
                path: finding.source_path.clone(),
                line: None,
                column: None,
                answer_id: finding.answer_id.clone(),
                phase: Some("validate".to_string()),
                suggestion: None,
                fix_example: None,
                docs_url: None,
                related_paths: None,
                caused_by: None,
            };
            if finding.code == "missing_review_by" {
                diagnostic.suggestion =
                    Some("Set `review_by = YYYY-MM-DD` for freshness tracking".to_string());
            }
            diagnostic
        })
        .collect()
}

fn emit_audit_report(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    format: AuditFormat,
    audit_date: chrono::NaiveDate,
    report: &AuditReport,
) {
    observability::emit_event_with_options(
        "governance",
        "audit",
        "ansorum.audit.completed",
        json!({
            "outcome": if report.has_errors() { "failed" } else { "passed" },
            "format": audit_format_name(format),
            "include_drafts": include_drafts,
            "audit_date": audit_date.to_string(),
            "root_dir": root_dir.display().to_string(),
            "config_file": config_file.display().to_string(),
            "report": report,
        }),
        DispatchMode::Sync,
        !format.is_json(),
    );
}

fn emit_audit_failure(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    format: AuditFormat,
    stage: &str,
    error: &str,
) {
    observability::emit_event_with_options(
        "governance",
        "audit",
        "ansorum.audit.completed",
        json!({
            "outcome": "failed",
            "format": audit_format_name(format),
            "include_drafts": include_drafts,
            "root_dir": root_dir.display().to_string(),
            "config_file": config_file.display().to_string(),
            "stage": stage,
            "error": error,
        }),
        DispatchMode::Sync,
        !format.is_json(),
    );
}

fn audit_format_name(format: AuditFormat) -> &'static str {
    match format {
        AuditFormat::Human => "human",
        AuditFormat::Json => "json",
        AuditFormat::JsonStream => "json_stream",
    }
}

fn print_json_report(
    root_dir: &Path,
    config_file: &Path,
    report: &AuditReport,
    diagnostics: Vec<Diagnostic>,
    duration: std::time::Duration,
    format: AuditFormat,
    threshold_exceeded: bool,
) -> Result<()> {
    let success = CommandSuccess {
        stage: "completed",
        diagnostics,
        artifacts: Default::default(),
        report: Some(
            serde_json::to_value(report)
                .map_err(|error| errors::Error::msg(format!("Failed to serialize audit report: {error}")))?,
        ),
    };
    let mut envelope = CommandReport::completed(
        "audit",
        if report.has_errors() || threshold_exceeded {
            ReportOutcome::Failed
        } else {
            ReportOutcome::Passed
        },
        root_dir,
        config_file,
        duration,
        success,
    );
    let compact = format.is_json_stream();
    if compact {
        let mut stream = diagnostics::ReportStreamContext::new("audit");
        envelope.attach_stream_event(&mut stream);
    }
    diagnostics::print_json_report(&envelope, compact)
}

fn print_failure_report(
    root_dir: &Path,
    config_file: &Path,
    format: AuditFormat,
    code: &str,
    phase: &str,
    message: &str,
    duration: std::time::Duration,
) -> Result<()> {
    if !format.is_json() {
        return Ok(());
    }

    let failure = CommandFailure::new(
        Diagnostic::error(code, message).with_phase(phase.to_string()),
    );
    let mut report = CommandReport::failure(
        "audit",
        Some(root_dir),
        Some(config_file),
        duration,
        failure,
    );
    let compact = format.is_json_stream();
    if compact {
        let mut stream = diagnostics::ReportStreamContext::new("audit");
        report.attach_stream_event(&mut stream);
    }
    diagnostics::print_json_report(&report, compact)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::audit;
    use crate::cli::{AuditFormat, FailOn};

    fn fixture_root(name: &str) -> std::path::PathBuf {
        env::current_dir().unwrap().join(name)
    }

    #[test]
    fn reference_project_audit_passes_in_human_and_json_modes() {
        let root = fixture_root("examples/reference-project");
        let config_file = root.join("config.toml");

        audit(&root, &config_file, false, AuditFormat::Human, FailOn::Error)
            .expect("human audit should pass");
        audit(&root, &config_file, false, AuditFormat::Json, FailOn::Error)
            .expect("json audit should pass");
    }

    #[test]
    fn invalid_reference_project_audit_fails() {
        let root = fixture_root("tests/fixtures/invalid/answers_audit");
        let config_file = root.join("config.toml");

        let err = audit(&root, &config_file, false, AuditFormat::Json, FailOn::Error)
            .expect_err("audit should fail");
        assert_eq!(err.to_string(), "Audit failed");
    }

    #[test]
    fn audit_warn_fixture_fails_when_fail_on_warn_is_enabled() {
        let root = fixture_root("tests/fixtures/invalid/answers_audit_warn");
        let config_file = root.join("config.toml");

        let err = audit(&root, &config_file, false, AuditFormat::Json, FailOn::Warn)
            .expect_err("audit should fail when warnings are promoted");
        assert_eq!(err.to_string(), "Audit failed");
    }

    #[test]
    fn audit_warn_fixture_passes_when_fail_on_error_is_enabled() {
        let root = fixture_root("tests/fixtures/invalid/answers_audit_warn");
        let config_file = root.join("config.toml");

        audit(&root, &config_file, false, AuditFormat::Json, FailOn::Error)
            .expect("audit should pass when warnings are allowed");
    }
}
