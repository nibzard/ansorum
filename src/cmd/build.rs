use std::fs;
use std::path::Path;

use crate::diagnostics::{CommandArtifacts, CommandFailure, CommandSuccess, Diagnostic};
use site::Site;

use crate::messages;

pub fn build(
    root_dir: &Path,
    config_file: &Path,
    base_url: Option<&str>,
    output_dir: Option<&Path>,
    force: bool,
    include_drafts: bool,
    minify: bool,
) -> Result<CommandSuccess, CommandFailure> {
    let mut site = Site::new(root_dir, config_file).map_err(|error| {
        CommandFailure::from_error("site_init_failed", error.to_string(), "load", error)
    })?;
    if let Some(output_dir) = output_dir {
        if !force && output_dir_requires_force(output_dir).map_err(CommandFailure::new)? {
            return Err(CommandFailure::new(
                Diagnostic::error(
                    "output_dir_exists",
                    format!("Directory '{}' already exists.", output_dir.display()),
                )
                .with_path(output_dir.display().to_string())
                .with_phase("preflight")
                .with_suggestion("Re-run with --force or choose a different --output-dir"),
            ));
        }

        site.set_output_path(output_dir);
    }
    if let Some(b) = base_url {
        site.set_base_url(b.to_string());
    }
    if include_drafts {
        site.include_drafts();
    }
    if minify {
        site.minify();
    }
    site.load().map_err(|error| {
        CommandFailure::from_error("site_load_failed", error.to_string(), "load", error)
    })?;
    let content = messages::collect_site_content_summary(&site);
    let diagnostics = messages::collect_ignored_page_diagnostics(&site);
    site.build().map_err(|error| {
        CommandFailure::from_error("site_build_failed", error.to_string(), "render", error)
    })?;

    Ok(CommandSuccess {
        stage: "completed",
        diagnostics,
        artifacts: CommandArtifacts {
            output_dir: Some(site.output_path.display().to_string()),
            content: Some(content),
        },
        report: None,
    })
}

fn output_dir_requires_force(output_dir: &Path) -> Result<bool, Diagnostic> {
    if !output_dir.exists() {
        return Ok(false);
    }

    let metadata = fs::metadata(output_dir).map_err(|error| {
        Diagnostic::error(
            "output_dir_unreadable",
            format!("Could not inspect output path '{}': {error}", output_dir.display()),
        )
        .with_path(output_dir.display().to_string())
        .with_phase("preflight")
        .with_suggestion("Check that --output-dir is accessible and readable")
    })?;

    if !metadata.is_dir() {
        return Ok(true);
    }

    let mut entries = fs::read_dir(output_dir).map_err(|error| {
        Diagnostic::error(
            "output_dir_unreadable",
            format!("Could not read output directory '{}': {error}", output_dir.display()),
        )
        .with_path(output_dir.display().to_string())
        .with_phase("preflight")
        .with_suggestion("Check that --output-dir is accessible and readable")
    })?;

    Ok(entries
        .next()
        .transpose()
        .map_err(|error| {
            Diagnostic::error(
                "output_dir_unreadable",
                format!("Could not read output directory '{}': {error}", output_dir.display()),
            )
            .with_path(output_dir.display().to_string())
            .with_phase("preflight")
            .with_suggestion("Check that --output-dir is accessible and readable")
        })?
        .is_some())
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::build;

    fn fixture_root(name: &str) -> std::path::PathBuf {
        env::current_dir().unwrap().join(name)
    }

    #[test]
    fn reference_project_build_passes() {
        let root = fixture_root("examples/reference-project");
        let config_file = root.join("config.toml");
        let output_dir = root.join("public-test-build");

        let success = build(&root, &config_file, None, Some(&output_dir), true, false, false)
            .expect("build should pass");
        assert_eq!(success.stage, "completed");
        assert_eq!(
            success.artifacts.output_dir.as_deref(),
            Some(output_dir.to_string_lossy().as_ref())
        );

        if output_dir.exists() {
            std::fs::remove_dir_all(output_dir).expect("cleanup build output");
        }
    }

    #[test]
    fn build_returns_output_dir_exists_diagnostic() {
        let root = fixture_root("examples/reference-project");
        let config_file = root.join("config.toml");
        let output_dir = root.join("public");

        let failure = build(&root, &config_file, None, Some(&output_dir), false, false, false)
            .expect_err("build should fail");
        assert_eq!(failure.diagnostics[0].code, "output_dir_exists");
    }

    #[test]
    fn build_allows_existing_empty_output_dir_without_force() {
        let root = fixture_root("examples/reference-project");
        let config_file = root.join("config.toml");
        let output_dir = std::env::temp_dir().join(format!(
            "ansorum-build-empty-output-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&output_dir).expect("create empty output dir");

        let success = build(&root, &config_file, None, Some(&output_dir), false, false, false)
            .expect("build should allow empty existing output dir");
        assert_eq!(success.stage, "completed");
        assert!(output_dir.join("index.html").exists());

        std::fs::remove_dir_all(output_dir).expect("cleanup build output");
    }

    #[test]
    fn invalid_template_fixture_returns_structured_diagnostic() {
        let root = fixture_root("tests/fixtures/invalid/template_parse_failure");
        let config_file = root.join("config.toml");

        let failure = build(&root, &config_file, None, None, false, false, false)
            .expect_err("build should fail");
        let diagnostic = failure
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "template_parse_failed")
            .expect("expected template parse diagnostic");
        assert_eq!(
            diagnostic.path.as_deref(),
            Some(root.join("templates/page.html").to_string_lossy().as_ref())
        );
        assert!(diagnostic.line.is_some());
        assert!(diagnostic.column.is_some());
    }
}
