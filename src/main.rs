use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Instant;

use chrono::Utc;
use cli::{Cli, Command, DiagnosticFormat};
use env_logger::Env;
use log;
use utils::net::{get_available_port, port_is_available};

use clap::{CommandFactory, Parser};
use time::UtcOffset;

mod cli;
mod cmd;
mod diagnostics;
mod fs_utils;
mod messages;
mod observability;
mod prompt;

fn canonicalize_root_dir(path: &Path) -> Result<PathBuf, diagnostics::CommandFailure> {
    path.canonicalize().map_err(|error| {
        diagnostics::CommandFailure::new(
            diagnostics::Diagnostic::error(
                "root_dir_invalid",
                format!("Could not resolve root dir `{}`: {error}", path.display()),
            )
            .with_path(path.display().to_string())
            .with_phase("preflight")
            .with_suggestion("Check that --root points to an existing directory"),
        )
    })
}

fn get_config_file_path(
    dir: &Path,
    config_path: Option<&Path>,
) -> Result<(PathBuf, PathBuf), diagnostics::CommandFailure> {
    if let Some(path) = config_path
        && path.is_absolute()
    {
        let config_file = path.canonicalize().map_err(|error| {
            diagnostics::CommandFailure::new(
                diagnostics::Diagnostic::error(
                    "config_path_invalid",
                    format!("Could not resolve config path `{}`: {error}", path.display()),
                )
                .with_path(path.display().to_string())
                .with_phase("preflight")
                .with_suggestion("Check that --config points to an existing config.toml file"),
            )
        })?;
        let root_dir = config_file.parent().unwrap_or_else(|| Path::new("/")).to_path_buf();
        return Ok((root_dir, config_file));
    }

    let (root_dir, config_path) = match config_path {
        Some(path) => {
            // User specified a config file, use it directly
            let root = dir.ancestors().find(|a| a.join(path).exists()).ok_or_else(|| {
                diagnostics::CommandFailure::new(
                    diagnostics::Diagnostic::error(
                        "config_not_found",
                        format!(
                            "`{}` not found in current directory or ancestors (current_dir={})",
                            path.display(),
                            dir.display()
                        ),
                    )
                    .with_path(path.display().to_string())
                    .with_phase("preflight")
                    .with_suggestion(
                        "Run ansorum from the project root or pass an absolute --config path",
                    ),
                )
            })?;
            (root, path.to_path_buf())
        }
        None => {
            let config = Path::new("config.toml");

            if let Some(root) = dir.ancestors().find(|a| a.join(config).exists()) {
                (root, config.to_path_buf())
            } else {
                return Err(diagnostics::CommandFailure::new(
                    diagnostics::Diagnostic::error(
                        "config_not_found",
                        format!(
                            "config.toml not found in current directory or ancestors (current_dir={})",
                            dir.display()
                        ),
                    )
                    .with_path(dir.display().to_string())
                    .with_phase("preflight")
                    .with_suggestion(
                        "Run ansorum from the project root or pass --config /path/to/config.toml",
                    ),
                ));
            }
        }
    };

    // if we got here we found root_dir so config file should exist so we could theoretically unwrap safely
    let config_file_uncanonicalized = root_dir.join(&config_path);
    let config_file = config_file_uncanonicalized.canonicalize().map_err(|error| {
        diagnostics::CommandFailure::new(
            diagnostics::Diagnostic::error(
                "config_path_invalid",
                format!(
                    "Could not resolve config path `{}`: {error}",
                    config_file_uncanonicalized.display()
                ),
            )
            .with_path(config_file_uncanonicalized.display().to_string())
            .with_phase("preflight")
            .with_suggestion("Check that the config file exists and is readable"),
        )
    })?;

    Ok((root_dir.to_path_buf(), config_file))
}

#[cfg(test)]
mod tests {
    use super::get_config_file_path;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn absolute_config_path_uses_config_parent_as_root() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).expect("epoch").as_nanos();
        let base = std::env::temp_dir().join(format!("ansorum-config-path-test-{unique}"));
        let project_root = base.join("site");
        fs::create_dir_all(&project_root).expect("create project root");
        let config_file = project_root.join("config.toml");
        fs::write(&config_file, "base_url = \"https://example.com\"\n").expect("write config");

        let work_dir = base.join("workspace").join("nested");
        fs::create_dir_all(&work_dir).expect("create work dir");

        let (root_dir, resolved_config) =
            get_config_file_path(&work_dir, Some(&config_file)).expect("resolve config path");

        assert_eq!(root_dir, project_root);
        assert_eq!(resolved_config, config_file);

        fs::remove_dir_all(base).expect("cleanup");
    }
}

// env-logger prints to stderr, so we detect color configuration by considering the stderr stream (and not stdout)
static SHOULD_COLOR_OUTPUT: LazyLock<anstream::ColorChoice> =
    LazyLock::new(|| anstream::AutoStream::choice(&std::io::stderr()));

fn main() {
    // Ensure that logging uses the info level for the main Ansorum binary by default.
    let env = Env::new().default_filter_or("ansorum=info");
    env_logger::Builder::from_env(env)
        .format(|f, record| {
            use std::io::Write;
            match record.level() {
                // INFO is used for normal CLI outputs, which we want to print with a little less noise
                log::Level::Info => {
                    writeln!(f, "{}", record.args())
                }
                _ => {
                    use anstyle::*;
                    let style = Style::new()
                        .fg_color(Some(Color::Ansi(match record.level() {
                            log::Level::Error => AnsiColor::Red,
                            log::Level::Warn => AnsiColor::Yellow,
                            log::Level::Info => AnsiColor::Green,
                            log::Level::Debug => AnsiColor::Cyan,
                            log::Level::Trace => AnsiColor::BrightBlack,
                        })))
                        .bold();
                    // Because the formatter erases the “terminal-ness” of stderr, we manually set the color behavior here.
                    let mut f = anstream::AutoStream::new(
                        f as &mut dyn std::io::Write,
                        *SHOULD_COLOR_OUTPUT,
                    );
                    writeln!(f, "{style}{:5}{style:#} {}", record.level().as_str(), record.args())
                }
            }
        })
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init { name, force, starter } => {
            if let Err(e) = cmd::create_new_project(&name, force, starter) {
                messages::unravel_errors("Failed to create the project", &e);
                std::process::exit(1);
            }
        }
        Command::Build { base_url, output_dir, force, drafts, minify, format, fail_on } => {
            let start = Instant::now();
            let cli_dir = match canonicalize_root_dir(&cli.root) {
                Ok(cli_dir) => cli_dir,
                Err(failure) => exit_build_or_check_failure("build", format, None, None, start, failure),
            };
            let (root_dir, config_file) = match get_config_file_path(&cli_dir, cli.config.as_deref()) {
                Ok(paths) => paths,
                Err(failure) => {
                    exit_build_or_check_failure("build", format, Some(&cli_dir), None, start, failure)
                }
            };
            if !format.is_json() {
                log::info!("Building site...");
            }
            match cmd::build(
                &root_dir,
                &config_file,
                base_url.as_deref(),
                output_dir.as_deref(),
                force,
                drafts,
                minify,
            ) {
                Ok(report) => finish_build_or_check_success(
                    "build",
                    format,
                    fail_on,
                    &root_dir,
                    &config_file,
                    start,
                    report,
                ),
                Err(failure) => exit_build_or_check_failure(
                    "build",
                    format,
                    Some(&root_dir),
                    Some(&config_file),
                    start,
                    failure,
                ),
            }
        }
        Command::Serve {
            interface,
            mut port,
            output_dir,
            force,
            base_url,
            drafts,
            open,
            store_html,
            fast,
            no_port_append,
            extra_watch_path,
            debounce,
            format,
        } => {
            let start = Instant::now();
            if port != 1111 && !port_is_available(interface, port) {
                exit_command_failure(
                    "serve",
                    format.is_json(),
                    format.is_json_stream(),
                    None,
                    None,
                    start,
                    "The requested port is not available",
                    diagnostics::CommandFailure::new(
                        diagnostics::Diagnostic::error(
                            "serve_port_unavailable",
                            format!("Requested port `{port}` is not available on interface `{interface}`"),
                        )
                        .with_phase("preflight")
                        .with_suggestion("Choose a different --port or stop the process using that port"),
                    ),
                );
            }

            if !port_is_available(interface, port) {
                port = get_available_port(interface, 1111).unwrap_or_else(|| {
                    exit_command_failure(
                        "serve",
                        format.is_json(),
                        format.is_json_stream(),
                        None,
                        None,
                        start,
                        "No port available",
                        diagnostics::CommandFailure::new(
                            diagnostics::Diagnostic::error(
                                "serve_no_port_available",
                                format!("No available port found on interface `{interface}` starting from 1111"),
                            )
                            .with_phase("preflight")
                            .with_suggestion("Free a local port or choose a different interface"),
                        ),
                    )
                });
            }

            let cli_dir = canonicalize_root_dir(&cli.root).unwrap_or_else(|failure| {
                exit_command_failure(
                    "serve",
                    format.is_json(),
                    format.is_json_stream(),
                    None,
                    None,
                    start,
                    "Failed to resolve root dir",
                    failure,
                )
            });
            let (root_dir, config_file) =
                get_config_file_path(&cli_dir, cli.config.as_deref()).unwrap_or_else(|failure| {
                    exit_command_failure(
                        "serve",
                        format.is_json(),
                        format.is_json_stream(),
                        Some(&cli_dir),
                        None,
                        start,
                        "Failed to resolve config file",
                        failure,
                    )
                });
            if !format.is_json() {
                log::info!("Building site...");
            }
            if let Err(e) = cmd::serve(
                &root_dir,
                interface,
                port,
                output_dir.as_deref(),
                force,
                base_url.as_deref(),
                &config_file,
                open,
                drafts,
                store_html,
                fast,
                no_port_append,
                UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
                extra_watch_path,
                debounce,
                format,
            ) {
                exit_command_failure(
                    "serve",
                    format.is_json(),
                    format.is_json_stream(),
                    Some(&root_dir),
                    Some(&config_file),
                    start,
                    "Failed to serve the site",
                    diagnostics::CommandFailure::from_error(
                        "serve_failed",
                        e.to_string(),
                        "serve",
                        e,
                    ),
                );
            }
        }
        Command::Check { drafts, skip_external_links, format, fail_on } => {
            let start = Instant::now();
            let cli_dir = match canonicalize_root_dir(&cli.root) {
                Ok(cli_dir) => cli_dir,
                Err(failure) => exit_build_or_check_failure("check", format, None, None, start, failure),
            };
            let (root_dir, config_file) = match get_config_file_path(&cli_dir, cli.config.as_deref()) {
                Ok(paths) => paths,
                Err(failure) => {
                    exit_build_or_check_failure("check", format, Some(&cli_dir), None, start, failure)
                }
            };
            if !format.is_json() {
                log::info!("Checking site...");
            }
            match cmd::check(&root_dir, &config_file, None, None, drafts, skip_external_links) {
                Ok(report) => finish_build_or_check_success(
                    "check",
                    format,
                    fail_on,
                    &root_dir,
                    &config_file,
                    start,
                    report,
                ),
                Err(failure) => exit_build_or_check_failure(
                    "check",
                    format,
                    Some(&root_dir),
                    Some(&config_file),
                    start,
                    failure,
                ),
            }
        }
        Command::Audit { drafts, format, fail_on } => {
            let start = Instant::now();
            let cli_dir = canonicalize_root_dir(&cli.root).unwrap_or_else(|failure| {
                exit_command_failure(
                    "audit",
                    format.is_json(),
                    format.is_json_stream(),
                    None,
                    None,
                    start,
                    "Failed to resolve root dir",
                    failure,
                )
            });
            let (root_dir, config_file) =
                get_config_file_path(&cli_dir, cli.config.as_deref()).unwrap_or_else(|failure| {
                    exit_command_failure(
                        "audit",
                        format.is_json(),
                        format.is_json_stream(),
                        Some(&cli_dir),
                        None,
                        start,
                        "Failed to resolve config file",
                        failure,
                    )
                });
            if !format.is_json() {
                log::info!("Auditing site as of {}...", Utc::now().date_naive());
            }
            if let Err(e) = cmd::audit(&root_dir, &config_file, drafts, format, fail_on) {
                if !format.is_json() {
                    messages::unravel_errors("Audit failed", &e);
                }
                std::process::exit(1);
            }
        }
        Command::Eval {
            drafts,
            fixtures,
            format,
            llm,
            model,
            api_base,
            min_pass_rate,
            min_llm_average,
            min_llm_score,
            require_llm,
            fail_on,
        } => {
            let start = Instant::now();
            let cli_dir = canonicalize_root_dir(&cli.root).unwrap_or_else(|failure| {
                exit_command_failure(
                    "eval",
                    format.is_json(),
                    format.is_json_stream(),
                    None,
                    None,
                    start,
                    "Failed to resolve root dir",
                    failure,
                )
            });
            let (root_dir, config_file) =
                get_config_file_path(&cli_dir, cli.config.as_deref()).unwrap_or_else(|failure| {
                    exit_command_failure(
                        "eval",
                        format.is_json(),
                        format.is_json_stream(),
                        Some(&cli_dir),
                        None,
                        start,
                        "Failed to resolve config file",
                        failure,
                    )
                });
            if !format.is_json() {
                log::info!("Running eval...");
            }
            if let Err(e) = cmd::eval(
                &root_dir,
                &config_file,
                drafts,
                &fixtures,
                format,
                llm,
                model.as_deref(),
                api_base.as_deref(),
                min_pass_rate,
                min_llm_average,
                min_llm_score,
                require_llm,
                fail_on,
            ) {
                if !format.is_json() {
                    messages::unravel_errors("Eval failed", &e);
                }
                std::process::exit(1);
            }
        }
        Command::Completion { shell } => {
            let cmd = &mut Cli::command();
            clap_complete::generate(shell, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
        }
    }
}

fn print_json_or_exit(command: &str, report: &diagnostics::CommandReport, compact: bool) {
    let mut report = report.clone();
    if compact {
        let mut stream = diagnostics::ReportStreamContext::new(command);
        report.attach_stream_event(&mut stream);
    }
    if let Err(error) = diagnostics::print_json_report(&report, compact) {
        messages::unravel_errors("Failed to print command report", &error);
        std::process::exit(1);
    }
}

fn finish_build_or_check_success(
    command: &'static str,
    format: DiagnosticFormat,
    fail_on: cli::FailOn,
    root_dir: &Path,
    config_file: &Path,
    start: Instant,
    mut success: diagnostics::CommandSuccess,
) -> ! {
    let threshold_exceeded = diagnostics::enforce_fail_on(&mut success, fail_on);
    let outcome = if threshold_exceeded {
        diagnostics::ReportOutcome::Failed
    } else {
        diagnostics::ReportOutcome::Passed
    };

    match format {
        DiagnosticFormat::Human => {
            let content = success.artifacts.content.as_ref().expect("command summary");
            match command {
                "build" => messages::print_build_success(content, &success.diagnostics),
                "check" => messages::print_check_success(content, &success.diagnostics),
                _ => {}
            }
            messages::report_elapsed_time(start);
        }
        DiagnosticFormat::Json | DiagnosticFormat::JsonStream => {
            let report = diagnostics::CommandReport::completed(
                command,
                outcome,
                root_dir,
                config_file,
                start.elapsed(),
                success,
            );
            print_json_or_exit(command, &report, format.is_json_stream());
        }
    }

    std::process::exit(if threshold_exceeded { 1 } else { 0 });
}

fn exit_build_or_check_failure(
    command: &'static str,
    format: DiagnosticFormat,
    root_dir: Option<&Path>,
    config_file: Option<&Path>,
    start: Instant,
    failure: diagnostics::CommandFailure,
) -> ! {
    match format {
        DiagnosticFormat::Human => {
            let prefix = match command {
                "build" => "Failed to build the site",
                "check" => "Failed to check the site",
                _ => "Command failed",
            };
            messages::print_command_failure(prefix, &failure);
        }
        DiagnosticFormat::Json | DiagnosticFormat::JsonStream => {
            let report = diagnostics::CommandReport::failure(
                command,
                root_dir,
                config_file,
                start.elapsed(),
                failure,
            );
            print_json_or_exit(command, &report, format.is_json_stream());
        }
    }

    std::process::exit(1);
}

fn exit_command_failure(
    command: &'static str,
    json_mode: bool,
    compact_json: bool,
    root_dir: Option<&Path>,
    config_file: Option<&Path>,
    start: Instant,
    human_prefix: &str,
    failure: diagnostics::CommandFailure,
) -> ! {
    if json_mode {
        let report = diagnostics::CommandReport::failure(
            command,
            root_dir,
            config_file,
            start.elapsed(),
            failure,
        );
        print_json_or_exit(command, &report, compact_json);
    } else {
        messages::print_command_failure(human_prefix, &failure);
    }

    std::process::exit(1);
}
