use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Instant;

use chrono::Utc;
use cli::{AuditFormat, Cli, Command, EvalFormat};
use env_logger::Env;
use errors::anyhow;
use log;
use utils::net::{get_available_port, port_is_available};

use clap::{CommandFactory, Parser};
use time::UtcOffset;

mod cli;
mod cmd;
mod fs_utils;
mod messages;
mod observability;
mod prompt;

fn get_config_file_path(dir: &Path, config_path: Option<&Path>) -> (PathBuf, PathBuf) {
    if let Some(path) = config_path
        && path.is_absolute()
    {
        let config_file = path.canonicalize().unwrap_or_else(|e| {
            messages::unravel_errors(
                &format!("Could not find canonical path of {}", path.display()),
                &e.into(),
            );
            std::process::exit(1);
        });
        let root_dir = config_file.parent().unwrap_or_else(|| Path::new("/")).to_path_buf();
        return (root_dir, config_file);
    }

    let (root_dir, config_path) = match config_path {
        Some(path) => {
            // User specified a config file, use it directly
            let root = dir.ancestors().find(|a| a.join(path).exists()).unwrap_or_else(|| {
                messages::unravel_errors(
                    "",
                    &anyhow!(
                        "{} not found in current directory or ancestors, current_dir is {}",
                        path.display(),
                        dir.display()
                    ),
                );
                std::process::exit(1);
            });
            (root, path.to_path_buf())
        }
        None => {
            let config = Path::new("config.toml");

            if let Some(root) = dir.ancestors().find(|a| a.join(config).exists()) {
                (root, config.to_path_buf())
            } else {
                messages::unravel_errors(
                    "",
                    &anyhow!(
                        "config.toml not found in current directory or ancestors, current_dir is {}",
                        dir.display()
                    ),
                );
                std::process::exit(1);
            }
        }
    };

    // if we got here we found root_dir so config file should exist so we could theoretically unwrap safely
    let config_file_uncanonicalized = root_dir.join(&config_path);
    let config_file = config_file_uncanonicalized.canonicalize().unwrap_or_else(|e| {
        messages::unravel_errors(
            &format!("Could not find canonical path of {}", config_file_uncanonicalized.display()),
            &e.into(),
        );
        std::process::exit(1);
    });

    (root_dir.to_path_buf(), config_file)
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

        let (root_dir, resolved_config) = get_config_file_path(&work_dir, Some(&config_file));

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
    let cli_dir: PathBuf = cli.root.canonicalize().unwrap_or_else(|e| {
        messages::unravel_errors(
            &format!("Could not find canonical path of root dir: {}", cli.root.display()),
            &e.into(),
        );
        std::process::exit(1);
    });

    match cli.command {
        Command::Init { name, force } => {
            if let Err(e) = cmd::create_new_project(&name, force) {
                messages::unravel_errors("Failed to create the project", &e);
                std::process::exit(1);
            }
        }
        Command::Build { base_url, output_dir, force, drafts, minify } => {
            log::info!("Building site...");
            let start = Instant::now();
            let (root_dir, config_file) = get_config_file_path(&cli_dir, cli.config.as_deref());
            match cmd::build(
                &root_dir,
                &config_file,
                base_url.as_deref(),
                output_dir.as_deref(),
                force,
                drafts,
                minify,
            ) {
                Ok(()) => messages::report_elapsed_time(start),
                Err(e) => {
                    messages::unravel_errors("Failed to build the site", &e);
                    std::process::exit(1);
                }
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
        } => {
            if port != 1111 && !port_is_available(interface, port) {
                log::error!("The requested port is not available");
                std::process::exit(1);
            }

            if !port_is_available(interface, port) {
                port = get_available_port(interface, 1111).unwrap_or_else(|| {
                    log::error!("No port available");
                    std::process::exit(1);
                });
            }

            let (root_dir, config_file) = get_config_file_path(&cli_dir, cli.config.as_deref());
            log::info!("Building site...");
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
            ) {
                messages::unravel_errors("Failed to serve the site", &e);
                std::process::exit(1);
            }
        }
        Command::Check { drafts, skip_external_links } => {
            log::info!("Checking site...");
            let start = Instant::now();
            let (root_dir, config_file) = get_config_file_path(&cli_dir, cli.config.as_deref());
            match cmd::check(&root_dir, &config_file, None, None, drafts, skip_external_links) {
                Ok(()) => messages::report_elapsed_time(start),
                Err(e) => {
                    messages::unravel_errors("Failed to check the site", &e);
                    std::process::exit(1);
                }
            }
        }
        Command::Audit { drafts, format } => {
            let (root_dir, config_file) = get_config_file_path(&cli_dir, cli.config.as_deref());
            if format == AuditFormat::Human {
                log::info!("Auditing site as of {}...", Utc::now().date_naive());
            }
            if let Err(e) = cmd::audit(&root_dir, &config_file, drafts, format) {
                if format == AuditFormat::Human {
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
        } => {
            let (root_dir, config_file) = get_config_file_path(&cli_dir, cli.config.as_deref());
            if format == EvalFormat::Human {
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
            ) {
                if format == EvalFormat::Human {
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
