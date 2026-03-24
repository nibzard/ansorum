use std::path::Path;

use chrono::Utc;
use errors::Result;
use serde_json::json;
use site::Site;
use site::answers::{AuditReport, audit_library};

use crate::cli::AuditFormat;
use crate::observability::{self, DispatchMode};

pub fn audit(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    format: AuditFormat,
) -> Result<()> {
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
            print_failure_report(format, "site_init_failed", &error.to_string())?;
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
        print_failure_report(format, "site_load_failed", &error.to_string())?;
        return Err(error);
    }

    let today = Utc::now().date_naive();
    let library = site.library.read().unwrap();
    let report = audit_library(&library, &site.answers, today);
    drop(library);

    emit_audit_report(root_dir, config_file, include_drafts, format, today, &report);

    match format {
        AuditFormat::Human => print_human_report(&report),
        AuditFormat::Json => print_json_report(&report)?,
    }

    if report.has_errors() {
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

fn emit_audit_report(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    format: AuditFormat,
    audit_date: chrono::NaiveDate,
    report: &AuditReport,
) {
    observability::emit_event(
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
    observability::emit_event(
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
    );
}

fn audit_format_name(format: AuditFormat) -> &'static str {
    match format {
        AuditFormat::Human => "human",
        AuditFormat::Json => "json",
    }
}

fn print_json_report(report: &AuditReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report).map_err(|error| {
        errors::Error::msg(format!("Failed to serialize audit report: {error}"))
    })?;
    println!("{json}");
    Ok(())
}

fn print_failure_report(format: AuditFormat, code: &str, message: &str) -> Result<()> {
    if format != AuditFormat::Json {
        return Ok(());
    }

    let report = AuditReport {
        summary: site::answers::AuditSummary { errors: 1, warnings: 0, infos: 0 },
        findings: vec![site::answers::AuditFinding {
            severity: site::answers::AuditSeverity::Error,
            code: code.to_string(),
            message: message.to_string(),
            answer_id: None,
            source_path: None,
        }],
    };
    print_json_report(&report)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::audit;
    use crate::cli::AuditFormat;

    fn fixture_root(name: &str) -> std::path::PathBuf {
        env::current_dir().unwrap().join(name)
    }

    #[test]
    fn reference_project_audit_passes_in_human_and_json_modes() {
        let root = fixture_root("test_site_answers");
        let config_file = root.join("config.toml");

        audit(&root, &config_file, false, AuditFormat::Human).expect("human audit should pass");
        audit(&root, &config_file, false, AuditFormat::Json).expect("json audit should pass");
    }

    #[test]
    fn invalid_reference_project_audit_fails() {
        let root = fixture_root("test_sites_invalid/answers_audit");
        let config_file = root.join("config.toml");

        let err =
            audit(&root, &config_file, false, AuditFormat::Json).expect_err("audit should fail");
        assert_eq!(err.to_string(), "Audit failed");
    }
}
