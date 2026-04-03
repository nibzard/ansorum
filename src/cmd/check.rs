use std::path::{Path, PathBuf};

use crate::diagnostics::{CommandArtifacts, CommandFailure, CommandSuccess};
use site::Site;

use crate::messages;

pub fn check(
    root_dir: &Path,
    config_file: &Path,
    base_path: Option<&str>,
    base_url: Option<&str>,
    include_drafts: bool,
    skip_external_links: bool,
) -> Result<CommandSuccess, CommandFailure> {
    let bp = base_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(root_dir));
    let mut site = Site::new(bp, config_file)
        .map_err(|error| CommandFailure::from_error("site_init_failed", error.to_string(), "load", error))?;
    // Force the checking of external links
    site.config.enable_check_mode();
    if let Some(b) = base_url {
        site.set_base_url(b.to_string());
    }
    if include_drafts {
        site.include_drafts();
    }
    if skip_external_links {
        site.skip_external_links_check();
    }
    site.load()
        .map_err(|error| CommandFailure::from_error("site_load_failed", error.to_string(), "load", error))?;
    let content = messages::collect_site_content_summary(&site);
    let mut diagnostics = messages::collect_orphan_page_diagnostics(&site);
    diagnostics.extend(messages::collect_ignored_page_diagnostics(&site));

    Ok(CommandSuccess {
        stage: "completed",
        diagnostics,
        artifacts: CommandArtifacts { output_dir: None, content: Some(content) },
        report: None,
    })
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::check;

    fn fixture_root(name: &str) -> std::path::PathBuf {
        env::current_dir().unwrap().join(name)
    }

    #[test]
    fn reference_project_check_passes() {
        let root = fixture_root("examples/reference-project");
        let config_file = root.join("config.toml");

        let success = check(&root, &config_file, None, None, false, true).expect("check should pass");
        assert_eq!(success.stage, "completed");
        assert_eq!(success.diagnostics.len(), 0);
    }

    #[test]
    fn duplicate_answer_id_fixture_returns_structured_diagnostic() {
        let root = fixture_root("tests/fixtures/invalid/answers_duplicate_id");
        let config_file = root.join("config.toml");

        let failure = check(&root, &config_file, None, None, false, true).expect_err("check should fail");
        assert!(failure.diagnostics.iter().any(|diagnostic| diagnostic.code == "answer_duplicate_id"));
    }

    #[test]
    fn missing_related_fixture_returns_structured_diagnostic() {
        let root = fixture_root("tests/fixtures/invalid/answers_missing_related");
        let config_file = root.join("config.toml");

        let failure = check(&root, &config_file, None, None, false, true).expect_err("check should fail");
        assert!(failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "answer_related_unknown_id"));
    }

    #[test]
    fn invalid_frontmatter_enum_fixture_returns_structured_diagnostic() {
        let root = fixture_root("tests/fixtures/invalid/answers_invalid_frontmatter_enum");
        let config_file = root.join("config.toml");

        let failure = check(&root, &config_file, None, None, false, true).expect_err("check should fail");
        let diagnostic = failure
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "frontmatter_invalid_enum")
            .expect("expected invalid enum diagnostic");
        assert_eq!(diagnostic.path.as_deref(), Some(root.join("content/refunds.md").to_string_lossy().as_ref()));
        assert!(diagnostic.line.is_some());
        assert!(diagnostic.column.is_some());
    }

    #[test]
    fn missing_required_field_fixture_returns_structured_diagnostic() {
        let root = fixture_root("tests/fixtures/invalid/answers_missing_required_field");
        let config_file = root.join("config.toml");

        let failure = check(&root, &config_file, None, None, false, true).expect_err("check should fail");
        let diagnostic = failure
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "frontmatter_missing_required_field")
            .expect("expected missing required field diagnostic");
        assert_eq!(diagnostic.path.as_deref(), Some(root.join("content/refunds.md").to_string_lossy().as_ref()));
    }
}
