use std::path::Path;

use chrono::Utc;
use errors::Result;
use site::Site;
use site::answers::{AuditReport, audit_library};

use crate::cli::AuditFormat;

pub fn audit(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    format: AuditFormat,
) -> Result<()> {
    let mut site = match Site::new(root_dir, config_file) {
        Ok(site) => site,
        Err(error) => {
            print_failure_report(format, "site_init_failed", &error.to_string())?;
            return Err(error);
        }
    };
    if include_drafts {
        site.include_drafts();
    }
    if let Err(error) = site.load() {
        print_failure_report(format, "site_load_failed", &error.to_string())?;
        return Err(error);
    }

    let today = Utc::now().date_naive();
    let library = site.library.read().unwrap();
    let report = audit_library(&library, &site.answers, today);
    drop(library);

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

fn print_json_report(report: &AuditReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| errors::Error::msg(format!("Failed to serialize audit report: {error}")))?;
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
