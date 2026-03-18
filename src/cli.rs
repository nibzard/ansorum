use std::net::IpAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap::ValueEnum;
use clap_complete::Shell;

#[derive(Parser)]
#[clap(
    name = "ansorum",
    bin_name = "ansorum",
    version,
    author,
    about = "An answer-first compiler for agent-readable and human-readable knowledge",
    after_help = "License: EUPL-1.2 <https://eupl.eu>, MIT for code existing before 0.22"
)]
pub struct Cli {
    /// Directory to use as root of project
    #[clap(short = 'r', long, default_value = ".")]
    pub root: PathBuf,

    /// Path to a config file other than config.toml in the root of project
    #[clap(short = 'c', long)]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum AuditFormat {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum EvalFormat {
    Human,
    Json,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new Ansorum project
    Init {
        /// Name of the project. Will create a new directory with that name in the current directory
        #[clap(default_value = ".")]
        name: String,

        /// Force creation of project even if directory is non-empty
        #[clap(short = 'f', long)]
        force: bool,
    },

    /// Deletes the output directory if there is one and builds the site
    Build {
        /// Force the base URL to be that value (defaults to the one in the config file)
        #[clap(short = 'u', long)]
        base_url: Option<String>,

        /// Outputs the generated site in the given path (by default 'public' dir in project root)
        #[clap(short = 'o', long)]
        output_dir: Option<PathBuf>,

        /// Force building the site even if output directory is non-empty
        #[clap(short = 'f', long)]
        force: bool,

        /// Include drafts when loading the site
        #[clap(long)]
        drafts: bool,

        /// Minify generated HTML files
        #[clap(long)]
        minify: bool,
    },

    /// Serve the site. Rebuild and reload on change automatically
    Serve {
        /// Interface to bind on
        #[clap(short = 'i', long, default_value = "127.0.0.1")]
        interface: IpAddr,

        /// Which port to use
        #[clap(short = 'p', long, default_value_t = 1111)]
        port: u16,

        /// Outputs assets of the generated site in the given path (by default 'public' dir in project root).
        /// HTML/XML will be stored in memory.
        #[clap(short = 'o', long)]
        output_dir: Option<PathBuf>,

        /// Force use of the directory for serving the site even if output directory is non-empty
        #[clap(long)]
        force: bool,

        /// Changes the base_url
        #[clap(short = 'u', long)]
        base_url: Option<String>,

        /// Include drafts when loading the site
        #[clap(long)]
        drafts: bool,

        /// Open site in the default browser
        #[clap(short = 'O', long)]
        open: bool,

        /// Also store HTML in the public/ folder (by default HTML is only stored in-memory)
        #[clap(long)]
        store_html: bool,

        /// Only rebuild the minimum on change - useful when working on a specific page/section
        #[clap(short = 'f', long)]
        fast: bool,

        /// Default append port to the base url.
        #[clap(long)]
        no_port_append: bool,

        /// Extra path to watch for changes, relative to the project root.
        #[clap(long)]
        extra_watch_path: Vec<String>,

        /// Debounce time in milliseconds for the file watcher (at least 1ms)
        #[clap(short = 'd', long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(1..))]
        debounce: u64,
    },

    /// Try to build the project without rendering it. Checks links
    Check {
        /// Include drafts when loading the site
        #[clap(long)]
        drafts: bool,
        /// Skip external links
        #[clap(long)]
        skip_external_links: bool,
    },

    /// Audit answer metadata, freshness, and machine-output quality
    Audit {
        /// Include drafts when loading the site
        #[clap(long)]
        drafts: bool,

        /// Output format
        #[clap(long, value_enum, default_value_t = AuditFormat::Human)]
        format: AuditFormat,
    },

    /// Evaluate retrieval, answer selection, and rubric-scored quality against fixtures
    Eval {
        /// Include drafts when loading the site
        #[clap(long)]
        drafts: bool,

        /// Path to the eval fixture file, relative to the project root
        #[clap(long, default_value = "eval/fixtures.yaml")]
        fixtures: PathBuf,

        /// Output format
        #[clap(long, value_enum, default_value_t = EvalFormat::Human)]
        format: EvalFormat,

        /// Enable LLM rubric scoring even if ansorum.eval.enabled is false
        #[clap(long)]
        llm: bool,

        /// Override the GPT-5.4 model for LLM rubric scoring
        #[clap(long)]
        model: Option<String>,

        /// Override the OpenAI Responses API base URL
        #[clap(long)]
        api_base: Option<String>,

        /// Require at least this overall case pass rate
        #[clap(long)]
        min_pass_rate: Option<f64>,

        /// Require at least this average LLM score across scored cases
        #[clap(long)]
        min_llm_average: Option<f64>,

        /// Require each LLM-scored case to meet at least this overall score
        #[clap(long)]
        min_llm_score: Option<f64>,

        /// Fail if any case does not receive an LLM score
        #[clap(long)]
        require_llm: bool,
    },

    /// Generate shell completion
    Completion {
        /// Shell to generate completion for
        #[clap(value_enum)]
        shell: Shell,
    },
}
