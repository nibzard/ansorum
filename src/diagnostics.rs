use std::path::Path;
use std::process;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use errors::{Error, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::cli::FailOn;

const DIAGNOSTICS_SCHEMA_VERSION: u8 = 1;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warn,
    Info,
}

impl DiagnosticSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<Vec<String>>,
}

#[allow(dead_code)]
impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Error, code, message)
    }

    pub fn warn(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Warn, code, message)
    }

    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Info, code, message)
    }

    fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            path: None,
            line: None,
            column: None,
            answer_id: None,
            phase: None,
            suggestion: None,
            fix_example: None,
            docs_url: None,
            related_paths: None,
            caused_by: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_answer_id(mut self, answer_id: impl Into<String>) -> Self {
        self.answer_id = Some(answer_id.into());
        self
    }

    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_fix_example(mut self, fix_example: impl Into<String>) -> Self {
        self.fix_example = Some(fix_example.into());
        self
    }

    pub fn with_docs_url(mut self, docs_url: impl Into<String>) -> Self {
        self.docs_url = Some(docs_url.into());
        self
    }

    pub fn with_related_paths(mut self, related_paths: Vec<String>) -> Self {
        if !related_paths.is_empty() {
            self.related_paths = Some(related_paths);
        }
        self
    }

    pub fn with_caused_by(mut self, caused_by: Vec<String>) -> Self {
        if !caused_by.is_empty() {
            self.caused_by = Some(caused_by);
        }
        self
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DiagnosticSummary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl DiagnosticSummary {
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        let mut summary = Self::default();
        for diagnostic in diagnostics {
            match diagnostic.severity {
                DiagnosticSeverity::Error => summary.errors += 1,
                DiagnosticSeverity::Warn => summary.warnings += 1,
                DiagnosticSeverity::Info => summary.infos += 1,
            }
        }
        summary
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SiteContentSummary {
    pub pages: usize,
    pub orphan_pages: usize,
    pub sections: usize,
    pub answer_records: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CommandArtifacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<SiteContentSummary>,
}

#[derive(Clone, Debug, Default)]
pub struct CommandSuccess {
    pub stage: &'static str,
    pub diagnostics: Vec<Diagnostic>,
    pub artifacts: CommandArtifacts,
    pub report: Option<JsonValue>,
}

#[derive(Clone, Debug)]
pub struct CommandFailure {
    pub diagnostics: Vec<Diagnostic>,
}

impl CommandFailure {
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self { diagnostics: vec![diagnostic] }
    }

    pub fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        assert!(!diagnostics.is_empty(), "CommandFailure requires at least one diagnostic");
        Self { diagnostics }
    }

    pub fn from_error(
        code: impl Into<String>,
        message: impl Into<String>,
        phase: impl Into<String>,
        error: Error,
    ) -> Self {
        Self::from_message_with_causes(code, message, phase, error_causes(&error))
    }

    pub fn from_message_with_causes(
        code: impl Into<String>,
        message: impl Into<String>,
        phase: impl Into<String>,
        causes: Vec<String>,
    ) -> Self {
        let code = code.into();
        let message = message.into();
        let phase = phase.into();
        let diagnostics = infer_error_diagnostics(&code, &message, &phase, &causes);
        if diagnostics.is_empty() {
            return Self::new(
                Diagnostic::error(code, message).with_phase(phase).with_caused_by(causes),
            );
        }

        Self::from_diagnostics(diagnostics)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportOutcome {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommandReport {
    pub schema_version: u8,
    pub product: &'static str,
    pub command: String,
    pub emitted_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    pub outcome: ReportOutcome,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
    pub duration_ms: u64,
    pub summary: DiagnosticSummary,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<CommandArtifacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<JsonValue>,
}

impl CommandReport {
    pub fn success(
        command: impl Into<String>,
        root_dir: &Path,
        config_file: &Path,
        duration: Duration,
        success: CommandSuccess,
    ) -> Self {
        Self::completed(command, ReportOutcome::Passed, root_dir, config_file, duration, success)
    }

    pub fn completed(
        command: impl Into<String>,
        outcome: ReportOutcome,
        root_dir: &Path,
        config_file: &Path,
        duration: Duration,
        success: CommandSuccess,
    ) -> Self {
        let summary = DiagnosticSummary::from_diagnostics(&success.diagnostics);
        Self {
            schema_version: DIAGNOSTICS_SCHEMA_VERSION,
            product: "ansorum",
            command: command.into(),
            emitted_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            stream_id: None,
            event_id: None,
            sequence: None,
            outcome,
            stage: success.stage.to_string(),
            root_dir: Some(root_dir.display().to_string()),
            config_file: Some(config_file.display().to_string()),
            duration_ms: duration_to_millis(duration),
            summary,
            diagnostics: success.diagnostics,
            artifacts: Some(success.artifacts),
            report: success.report,
        }
    }

    pub fn failure(
        command: impl Into<String>,
        root_dir: Option<&Path>,
        config_file: Option<&Path>,
        duration: Duration,
        failure: CommandFailure,
    ) -> Self {
        let diagnostics = failure.diagnostics;
        let stage = diagnostics
            .first()
            .and_then(|diagnostic| diagnostic.phase.clone())
            .unwrap_or_else(|| "failed".to_string());
        let summary = DiagnosticSummary::from_diagnostics(&diagnostics);
        Self {
            schema_version: DIAGNOSTICS_SCHEMA_VERSION,
            product: "ansorum",
            command: command.into(),
            emitted_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            stream_id: None,
            event_id: None,
            sequence: None,
            outcome: ReportOutcome::Failed,
            stage,
            root_dir: root_dir.map(|path| path.display().to_string()),
            config_file: config_file.map(|path| path.display().to_string()),
            duration_ms: duration_to_millis(duration),
            summary,
            diagnostics,
            artifacts: None,
            report: None,
        }
    }

    pub fn attach_stream_event(&mut self, stream: &mut ReportStreamContext) {
        let sequence = stream.next_sequence;
        stream.next_sequence += 1;
        self.stream_id = Some(stream.stream_id.clone());
        self.sequence = Some(sequence);
        self.event_id = Some(format!("{}:{sequence}", stream.stream_id));
    }
}

#[derive(Clone, Debug)]
pub struct ReportStreamContext {
    stream_id: String,
    next_sequence: u64,
}

impl ReportStreamContext {
    pub fn new(command: &str) -> Self {
        Self {
            stream_id: format!(
                "{}-{}-{}",
                command,
                process::id(),
                Utc::now().timestamp_millis()
            ),
            next_sequence: 1,
        }
    }
}

pub fn print_json_report(report: &CommandReport, compact: bool) -> Result<()> {
    let json = if compact {
        serde_json::to_string(report)
    } else {
        serde_json::to_string_pretty(report)
    }
    .map_err(|error| Error::msg(format!("Failed to serialize command report: {error}")))?;
    println!("{json}");
    Ok(())
}

pub fn enforce_fail_on(success: &mut CommandSuccess, fail_on: FailOn) -> bool {
    let summary = DiagnosticSummary::from_diagnostics(&success.diagnostics);
    if !fail_on.threshold_exceeded(summary.errors, summary.warnings) || fail_on == FailOn::Error {
        return false;
    }

    success.diagnostics.push(
        Diagnostic::error(
            "warn_threshold_exceeded",
            "Command completed with warnings, but `--fail-on warn` requires a warning-free run",
        )
        .with_phase(success.stage)
        .with_suggestion("Resolve warning diagnostics or re-run with `--fail-on error`"),
    );
    true
}

fn duration_to_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn error_causes(error: &Error) -> Vec<String> {
    let mut causes = Vec::new();
    let mut source = error.source();
    while let Some(cause) = source {
        causes.push(cause.to_string());
        source = cause.source();
    }
    causes
}

pub fn collect_error_causes(error: &Error) -> Vec<String> {
    error_causes(error)
}

fn infer_error_diagnostics(
    default_code: &str,
    message: &str,
    phase: &str,
    causes: &[String],
) -> Vec<Diagnostic> {
    if let Some(diagnostics) = parse_template_failure_text(message, phase) {
        return diagnostics;
    }
    for cause in causes {
        if let Some(diagnostics) = parse_template_failure_text(cause, phase) {
            return diagnostics;
        }
    }
    parse_answer_validation(message, phase)
        .or_else(|| parse_front_matter_failure(message, phase, causes))
        .or_else(|| parse_template_parse_failure(message, phase, causes))
        .or_else(|| parse_sidecar_failure(message, phase))
        .or_else(|| parse_index_conflict(message, phase))
        .unwrap_or_else(|| {
            vec![Diagnostic::error(default_code, message.to_string())
                .with_phase(phase.to_string())
                .with_caused_by(causes.to_vec())]
        })
}

fn parse_answer_validation(message: &str, phase: &str) -> Option<Vec<Diagnostic>> {
    let body = message.strip_prefix("Answer validation failed:\n- ")?;
    let mut diagnostics = Vec::new();
    for bullet in body.split("\n- ") {
        if let Some(diagnostic) = parse_answer_validation_bullet(bullet, phase) {
            diagnostics.push(diagnostic);
        } else {
            diagnostics.push(
                Diagnostic::error("answer_validation_failed", bullet.to_string())
                    .with_phase(phase.to_string()),
            );
        }
    }
    Some(diagnostics)
}

fn parse_answer_validation_bullet(bullet: &str, phase: &str) -> Option<Diagnostic> {
    if let Some(rest) = bullet.strip_prefix("Duplicate answer id `") {
        let (id, paths) = rest.split_once("` found in ")?;
        let mut path_list = paths.split(", ").map(ToOwned::to_owned).collect::<Vec<_>>();
        let path = path_list.first().cloned();
        if !path_list.is_empty() {
            path_list.remove(0);
        }
        let mut diagnostic = Diagnostic::error(
            "answer_duplicate_id",
            format!("Duplicate answer id `{id}` found in multiple files"),
        )
        .with_phase(phase.to_string())
        .with_suggestion("Rename one of the answer ids so each answer has a unique `id`");
        if let Some(path) = path {
            diagnostic = diagnostic.with_path(path);
        }
        if !path_list.is_empty() {
            diagnostic = diagnostic.with_related_paths(path_list);
        }
        return Some(diagnostic);
    }

    if let Some(rest) = bullet.strip_prefix("Duplicate canonical question `") {
        let (question, paths) = rest.split_once("` found in ")?;
        let mut path_list = paths.split(", ").map(ToOwned::to_owned).collect::<Vec<_>>();
        let path = path_list.first().cloned();
        if !path_list.is_empty() {
            path_list.remove(0);
        }
        let mut diagnostic = Diagnostic::error(
            "answer_duplicate_canonical_question",
            format!("Duplicate canonical question `{question}` found in multiple files"),
        )
        .with_phase(phase.to_string())
        .with_suggestion("Keep each canonical question attached to only one answer");
        if let Some(path) = path {
            diagnostic = diagnostic.with_path(path);
        }
        if !path_list.is_empty() {
            diagnostic = diagnostic.with_related_paths(path_list);
        }
        return Some(diagnostic);
    }

    if let Some(rest) = bullet.strip_prefix("Duplicate retrieval alias `") {
        let (alias, paths) = rest.split_once("` found in ")?;
        let mut path_list = paths.split(", ").map(ToOwned::to_owned).collect::<Vec<_>>();
        let path = path_list.first().cloned();
        if !path_list.is_empty() {
            path_list.remove(0);
        }
        let mut diagnostic = Diagnostic::error(
            "answer_duplicate_retrieval_alias",
            format!("Duplicate retrieval alias `{alias}` found in multiple files"),
        )
        .with_phase(phase.to_string())
        .with_suggestion("Keep each retrieval alias attached to only one answer");
        if let Some(path) = path {
            diagnostic = diagnostic.with_path(path);
        }
        if !path_list.is_empty() {
            diagnostic = diagnostic.with_related_paths(path_list);
        }
        return Some(diagnostic);
    }

    let (path, detail) = bullet.split_once(": ")?;
    if let Some(rest) = detail.strip_prefix("`related` references unknown answer id `") {
        let related_id = rest.strip_suffix('`')?;
        return Some(
            Diagnostic::error(
                "answer_related_unknown_id",
                format!("`related` references unknown answer id `{related_id}`"),
            )
            .with_path(path.to_string())
            .with_phase(phase.to_string())
            .with_suggestion("Point `related` to an existing answer id or remove the stale link"),
        );
    }

    if let Some(rest) = detail.strip_prefix("`related` cannot reference the answer itself (`") {
        let related_id = rest.strip_suffix("`)")?;
        return Some(
            Diagnostic::error(
                "answer_related_self_reference",
                format!("`related` cannot reference the answer itself (`{related_id}`)"),
            )
            .with_path(path.to_string())
            .with_phase(phase.to_string())
            .with_suggestion("Remove the self-reference from `related`"),
        );
    }

    if let Some(rest) = detail.strip_prefix("`canonical_questions` contains duplicate entry `") {
        let entry = rest.strip_suffix('`')?;
        return Some(
            Diagnostic::error(
                "answer_duplicate_question_entry",
                format!("`canonical_questions` contains duplicate entry `{entry}`"),
            )
            .with_path(path.to_string())
            .with_phase(phase.to_string())
            .with_suggestion("Remove the duplicate value from `canonical_questions`"),
        );
    }

    if let Some(rest) = detail.strip_prefix("`retrieval_aliases` contains duplicate entry `") {
        let entry = rest.strip_suffix('`')?;
        return Some(
            Diagnostic::error(
                "answer_duplicate_retrieval_alias_entry",
                format!("`retrieval_aliases` contains duplicate entry `{entry}`"),
            )
            .with_path(path.to_string())
            .with_phase(phase.to_string())
            .with_suggestion("Remove the duplicate value from `retrieval_aliases`"),
        );
    }

    if let Some(rest) = detail.strip_prefix("`related` contains duplicate entry `") {
        let entry = rest.strip_suffix('`')?;
        return Some(
            Diagnostic::error(
                "answer_duplicate_related_link",
                format!("`related` contains duplicate entry `{entry}`"),
            )
            .with_path(path.to_string())
            .with_phase(phase.to_string())
            .with_suggestion("Keep each `related` answer id at most once"),
        );
    }

    None
}

fn parse_front_matter_failure(
    message: &str,
    phase: &str,
    causes: &[String],
) -> Option<Vec<Diagnostic>> {
    if !message.starts_with("Error when parsing front matter") {
        return None;
    }
    let start = message.find('`')?;
    let end = message.rfind('`')?;
    if end <= start {
        return None;
    }
    let path = message[start + 1..end].to_string();
    let cause = causes.first().cloned().unwrap_or_else(|| message.to_string());

    if let Some(field) = cause
        .strip_prefix("An answer page requires `")
        .and_then(|value| value.strip_suffix('`'))
    {
        return Some(vec![
            Diagnostic::error(
                "frontmatter_missing_required_field",
                format!("Answer front matter requires `{field}`"),
            )
            .with_path(path)
            .with_phase(phase.to_string())
            .with_suggestion(format!("Add `{field} = ...` to the page front matter")),
        ]);
    }

    if cause.starts_with("TOML parse error at line ") {
        let (line, column) = parse_line_column(&cause)?;
        let mut diagnostic = if let Some(rest) = cause.split("unknown variant `").nth(1) {
            let (variant, expected) = rest.split_once("`, expected one of ")?;
            Diagnostic::error(
                "frontmatter_invalid_enum",
                format!("Invalid front matter value `{variant}`"),
            )
            .with_suggestion(format!("Use one of: {}", expected.replace('`', "").trim()))
        } else {
            Diagnostic::error("frontmatter_parse_failed", "Front matter could not be parsed")
                .with_suggestion("Fix the TOML syntax near the reported line and column")
        };
        diagnostic = diagnostic
            .with_path(path)
            .with_line(line)
            .with_column(column)
            .with_phase(phase.to_string())
            .with_caused_by(vec![cause]);
        return Some(vec![diagnostic]);
    }

    Some(vec![
        Diagnostic::error("frontmatter_parse_failed", cause)
            .with_path(path)
            .with_phase(phase.to_string())
            .with_suggestion("Fix the page front matter and re-run the command"),
    ])
}

fn parse_template_parse_failure(
    message: &str,
    phase: &str,
    causes: &[String],
) -> Option<Vec<Diagnostic>> {
    if message != "Error parsing templates from the /templates directory"
        && message != "Error parsing templates from themes"
    {
        return None;
    }

    let cause = causes.first()?.clone();
    parse_template_failure_text(&cause, phase)
        .map(|mut diagnostics| {
            diagnostics[0] = diagnostics[0].clone().with_caused_by(vec![cause]);
            diagnostics
        })
}

fn parse_sidecar_failure(message: &str, phase: &str) -> Option<Vec<Diagnostic>> {
    let path = message
        .strip_prefix("Failed to parse structured-data sidecar `")?
        .split("`: ")
        .next()?
        .to_string();
    let details = message.split("`: ").nth(1)?.to_string();
    let mut diagnostic = Diagnostic::error("schema_sidecar_invalid_json", details.clone())
        .with_path(path)
        .with_phase(phase.to_string())
        .with_suggestion("Fix the JSON syntax in the structured-data sidecar");
    if let Some((line, column)) = parse_line_column(&details) {
        diagnostic = diagnostic.with_line(line).with_column(column);
    }
    Some(vec![diagnostic])
}

fn parse_index_conflict(message: &str, phase: &str) -> Option<Vec<Diagnostic>> {
    let path = message
        .split(" in \"")
        .nth(1)
        .and_then(|value| value.strip_suffix('"'))?
        .to_string();
    Some(vec![
        Diagnostic::error("content_index_conflict", message.to_string())
            .with_path(path)
            .with_phase(phase.to_string())
            .with_suggestion("Rename `index.md` or remove the conflicting `_index.md` section"),
    ])
}

fn parse_template_failure_text(text: &str, phase: &str) -> Option<Vec<Diagnostic>> {
    let path = text
        .split("Failed to parse \"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .map(ToOwned::to_owned)?;
    let (line, column) = parse_arrow_line_column(text).unwrap_or((0, 0));
    let details = text
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("= "))
        .unwrap_or("Template syntax error");
    let mut diagnostic = Diagnostic::error("template_parse_failed", details.to_string())
        .with_path(path)
        .with_phase(phase.to_string())
        .with_suggestion("Fix the Tera template syntax at the reported location");
    if line > 0 {
        diagnostic = diagnostic.with_line(line);
    }
    if column > 0 {
        diagnostic = diagnostic.with_column(column);
    }
    Some(vec![diagnostic])
}


fn parse_line_column(text: &str) -> Option<(usize, usize)> {
    let line = text
        .split("line ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .trim_end_matches(',')
        .parse()
        .ok()?;
    let column = text
        .split("column ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .trim_end_matches(',')
        .parse()
        .ok()?;
    Some((line, column))
}

fn parse_arrow_line_column(text: &str) -> Option<(usize, usize)> {
    let marker = text.lines().find(|line| line.trim_start().starts_with("-->"))?;
    let coords = marker.trim().strip_prefix("-->")?.trim();
    let (line, column) = coords.split_once(':')?;
    Some((line.trim().parse().ok()?, column.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{CommandFailure, CommandReport, CommandSuccess, Diagnostic, DiagnosticSeverity};
    use crate::cli::FailOn;

    #[test]
    fn failure_report_counts_error_diagnostics() {
        let failure = CommandFailure::new(
            Diagnostic::error("config_not_found", "config.toml not found").with_phase("preflight"),
        );
        let report = CommandReport::failure("build", None, None, std::time::Duration::ZERO, failure);

        assert_eq!(report.summary.errors, 1);
        assert_eq!(report.summary.warnings, 0);
        assert_eq!(report.stage, "preflight");
    }

    #[test]
    fn success_report_counts_warning_diagnostics() {
        let success = CommandSuccess {
            stage: "completed",
            diagnostics: vec![Diagnostic::warn("ignored_page", "page ignored")],
            artifacts: Default::default(),
            report: None,
        };
        let report = CommandReport::success(
            "check",
            Path::new("/tmp/site"),
            Path::new("/tmp/site/config.toml"),
            std::time::Duration::ZERO,
            success,
        );

        assert_eq!(report.summary.errors, 0);
        assert_eq!(report.summary.warnings, 1);
        assert_eq!(report.diagnostics[0].severity, DiagnosticSeverity::Warn);
    }

    #[test]
    fn parses_front_matter_enum_failures_into_structured_diagnostics() {
        let message = "Error when parsing front matter of section `/tmp/site/content/refunds.md`";
        let failure = CommandFailure::from_message_with_causes(
            "site_load_failed",
            message,
            "load",
            vec![String::from(
                "TOML parse error at line 18, column 17\n   |\n18 | ai_visibility = \"bad-value\"\n   |                 ^^^^^^^^^^^\nunknown variant `bad-value`, expected one of `public`, `hidden`, `summary_only`\n",
            )],
        );

        assert_eq!(failure.diagnostics[0].code, "frontmatter_invalid_enum");
        assert_eq!(failure.diagnostics[0].path.as_deref(), Some("/tmp/site/content/refunds.md"));
        assert_eq!(failure.diagnostics[0].line, Some(18));
        assert_eq!(failure.diagnostics[0].column, Some(17));
    }

    #[test]
    fn parses_duplicate_answer_id_into_structured_diagnostics() {
        let failure = CommandFailure::from_message_with_causes(
            "site_load_failed",
            "Answer validation failed:\n- Duplicate answer id `duplicate-answer` found in /tmp/site/content/one.md, /tmp/site/content/two.md",
            "load",
            Vec::new(),
        );

        assert_eq!(failure.diagnostics.len(), 1);
        assert_eq!(failure.diagnostics[0].code, "answer_duplicate_id");
        assert_eq!(failure.diagnostics[0].path.as_deref(), Some("/tmp/site/content/one.md"));
        assert_eq!(
            failure.diagnostics[0].related_paths.as_ref().expect("related paths"),
            &vec!["/tmp/site/content/two.md".to_string()]
        );
    }

    #[test]
    fn parses_missing_related_answer_into_structured_diagnostics() {
        let failure = CommandFailure::from_message_with_causes(
            "site_load_failed",
            "Answer validation failed:\n- /tmp/site/content/refunds.md: `related` references unknown answer id `missing-answer`",
            "load",
            Vec::new(),
        );

        assert_eq!(failure.diagnostics[0].code, "answer_related_unknown_id");
        assert_eq!(
            failure.diagnostics[0].message,
            "`related` references unknown answer id `missing-answer`"
        );
        assert_eq!(failure.diagnostics[0].path.as_deref(), Some("/tmp/site/content/refunds.md"));
    }

    #[test]
    fn parses_missing_required_frontmatter_field_into_structured_diagnostics() {
        let failure = CommandFailure::from_message_with_causes(
            "site_load_failed",
            "Error when parsing front matter of section `/tmp/site/content/refunds.md`",
            "load",
            vec!["An answer page requires `ai_visibility`".to_string()],
        );

        assert_eq!(failure.diagnostics[0].code, "frontmatter_missing_required_field");
        assert_eq!(
            failure.diagnostics[0].message,
            "Answer front matter requires `ai_visibility`"
        );
    }

    #[test]
    fn parses_template_parse_failures_into_structured_diagnostics() {
        let failure = CommandFailure::from_message_with_causes(
            "serve_rebuild_failed",
            "\n* Failed to parse \"/tmp/site/templates/page.html\"\n --> 2:1\n  |\n2 | \n  | ^---\n  |\n  = expected `or`, `and`, `not`, `<=`, `>=`, `<`, `>`, `==`, `!=`, `+`, `-`, `*`, `/`, `%`, a filter, or a variable end (`}}`)",
            "rebuild",
            Vec::new(),
        );

        assert_eq!(failure.diagnostics[0].code, "template_parse_failed");
        assert_eq!(failure.diagnostics[0].path.as_deref(), Some("/tmp/site/templates/page.html"));
        assert_eq!(failure.diagnostics[0].line, Some(2));
        assert_eq!(failure.diagnostics[0].column, Some(1));
    }

    #[test]
    fn parses_wrapped_template_parse_failures_from_causes() {
        let failure = CommandFailure::from_message_with_causes(
            "site_load_failed",
            "Error parsing templates from the /templates directory",
            "load",
            vec![String::from(
                "\n* Failed to parse \"/tmp/site/templates/page.html\"\n --> 7:13\n  |\n7 | {% if page.title %\n  |             ^---\n  |\n  = expected a variable end (`}}`)",
            )],
        );

        assert_eq!(failure.diagnostics[0].code, "template_parse_failed");
        assert_eq!(failure.diagnostics[0].path.as_deref(), Some("/tmp/site/templates/page.html"));
        assert_eq!(failure.diagnostics[0].line, Some(7));
        assert_eq!(failure.diagnostics[0].column, Some(13));
        assert_eq!(failure.diagnostics[0].phase.as_deref(), Some("load"));
    }

    #[test]
    fn parses_invalid_sidecar_json_into_structured_diagnostics() {
        let failure = CommandFailure::from_message_with_causes(
            "site_load_failed",
            "Failed to parse structured-data sidecar `/tmp/site/content/refunds.schema.json`: EOF while parsing an object at line 9 column 0",
            "load",
            Vec::new(),
        );

        assert_eq!(failure.diagnostics[0].code, "schema_sidecar_invalid_json");
        assert_eq!(
            failure.diagnostics[0].path.as_deref(),
            Some("/tmp/site/content/refunds.schema.json")
        );
        assert_eq!(failure.diagnostics[0].line, Some(9));
        assert_eq!(failure.diagnostics[0].column, Some(0));
    }

    #[test]
    fn falls_back_to_default_diagnostic_when_message_is_unknown() {
        let failure = CommandFailure::from_message_with_causes(
            "site_load_failed",
            "something unexpected happened",
            "load",
            vec!["root cause".to_string()],
        );

        assert_eq!(failure.diagnostics[0].code, "site_load_failed");
        assert_eq!(failure.diagnostics[0].phase.as_deref(), Some("load"));
        assert_eq!(
            failure.diagnostics[0].caused_by.as_ref().expect("caused_by"),
            &vec!["root cause".to_string()]
        );
    }

    #[test]
    fn enforce_fail_on_warn_promotes_warnings_to_error() {
        let mut success = CommandSuccess {
            stage: "completed",
            diagnostics: vec![Diagnostic::warn("orphan_page", "Page is not linked from any section")],
            artifacts: Default::default(),
            report: None,
        };

        let threshold_exceeded = super::enforce_fail_on(&mut success, FailOn::Warn);

        assert!(threshold_exceeded);
        assert_eq!(success.diagnostics.len(), 2);
        assert_eq!(success.diagnostics[1].code, "warn_threshold_exceeded");
        assert_eq!(success.diagnostics[1].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn enforce_fail_on_error_does_not_promote_warnings() {
        let mut success = CommandSuccess {
            stage: "completed",
            diagnostics: vec![Diagnostic::warn("orphan_page", "Page is not linked from any section")],
            artifacts: Default::default(),
            report: None,
        };

        let threshold_exceeded = super::enforce_fail_on(&mut success, FailOn::Error);

        assert!(!threshold_exceeded);
        assert_eq!(success.diagnostics.len(), 1);
        assert_eq!(success.diagnostics[0].severity, DiagnosticSeverity::Warn);
    }
}
