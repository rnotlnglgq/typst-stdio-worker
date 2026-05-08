mod interactive;
mod pipe;
mod worker;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::i18n;
use crate::template::TemplateKind;
use crate::world::TypstBotWorld;

#[derive(Parser)]
#[command(
    name = "typst-stdio-worker",
    about = i18n::about(),
    version = env!("CARGO_PKG_VERSION"),
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Additional font directories to load (can be repeated).
    #[arg(long, short = 'f', help = i18n::help_font_path(), global = true)]
    pub font_path: Vec<PathBuf>,

    /// Same as `typst compile --package-path` / `TYPST_PACKAGE_PATH`.
    #[arg(
        long = "package-path",
        short = 'l',
        value_name = "DIR",
        env = "TYPST_PACKAGE_PATH",
        help = i18n::help_package_path(),
        global = true
    )]
    pub package_path: Option<PathBuf>,

    /// Same as `typst compile --package-cache-path` / `TYPST_PACKAGE_CACHE_PATH`.
    #[arg(
        long = "package-cache-path",
        short = 'p',
        value_name = "DIR",
        env = "TYPST_PACKAGE_CACHE_PATH",
        help = i18n::help_package_cache_path(),
        global = true
    )]
    pub package_cache_path: Option<PathBuf>,

    /// Allow downloading missing packages from the Typst package registry.
    #[arg(long, help = i18n::help_allow_download(), global = true)]
    pub allow_download: bool,

    /// PNG pixel scale factor (typical: 1.0–4.0).
    #[arg(long, short = 's', default_value = "4.0", help = i18n::help_scale(), global = true)]
    pub scale: f32,

    /// Maximum number of pages to render.
    #[arg(long, default_value = "1", help = i18n::help_max_pages(), global = true)]
    pub max_pages: usize,

    /// Maximum total pixels (width*height across all pages).
    #[arg(long, default_value = "100000000", help = i18n::help_max_pixels(), global = true)]
    pub max_pixels: u64,

    /// Maximum input size in bytes (1 MiB).
    #[arg(long, default_value = "1048576", help = i18n::help_max_input_size(), global = true)]
    pub max_input_size: usize,

    /// Log level filter.
    #[arg(long, default_value = "info", help = i18n::help_log_level(), global = true)]
    pub log_level: String,

    /// Log font blob aggregate size and per-render PNG dimensions / encoded size.
    #[arg(long, help = i18n::help_meter(), global = true)]
    pub meter: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Long-running NDJSON worker (stdin/stdout protocol).
    #[command(about = i18n::cmd_worker_about())]
    Worker {
        /// Exit after N compilations (0 = unlimited).
        #[arg(long, default_value = "0", help = i18n::help_max_compilations())]
        max_compilations: u64,
    },

    /// Interactive REPL: type source, Enter to compile.
    #[command(about = i18n::cmd_interactive_about())]
    Interactive {
        /// Open the rendered PNG with xdg-open after compilation.
        #[arg(long, help = i18n::help_open())]
        open: bool,

        /// Template to apply.
        #[arg(long, value_enum, default_value_t = TemplateKind::Raw, help = i18n::help_template())]
        template: TemplateKind,

        /// Exit after N compilations (0 = unlimited).
        #[arg(long, default_value = "0", help = i18n::help_max_compilations())]
        max_compilations: u64,
    },

    /// One-shot pipe mode: stdin -> compile -> stdout PNG.
    #[command(about = i18n::cmd_pipe_about())]
    Pipe {
        /// Template to apply.
        #[arg(long, value_enum, default_value_t = TemplateKind::Raw, help = i18n::help_template())]
        template: TemplateKind,
    },
}

/// Fallback package cache when `--package-cache-path` / `TYPST_PACKAGE_CACHE_PATH` are unset.
fn default_package_cache_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return Some(PathBuf::from(xdg).join("typst/packages"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".cache/typst/packages"));
    }
    None
}

/// Main entry point: parse CLI, initialize subsystems, dispatch to mode handler.
pub fn run() -> ExitCode {
    let cli = Cli::parse();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cli.log_level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let shutdown = std::sync::Arc::new(AtomicBool::new(false));
    for sig in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT] {
        if let Err(e) = signal_hook::flag::register(sig, shutdown.clone()) {
            tracing::warn!(error = %e, "{}", i18n::log_signal_register_failed());
        }
    }

    let package_dir = cli
        .package_cache_path
        .clone()
        .or_else(default_package_cache_dir);
    if let Some(ref dir) = package_dir {
        tracing::info!(path = %dir.display(), "{}", i18n::log_using_package_cache());
    } else {
        tracing::debug!("{}", i18n::log_no_package_cache());
    }

    if let Some(ref dir) = cli.package_path {
        tracing::info!(path = %dir.display(), "{}", i18n::log_using_package_path());
    } else {
        tracing::debug!("{}", i18n::log_no_package_path());
    }

    tracing::info!("{}", i18n::log_loading_fonts());
    let (mut world, font_count) = TypstBotWorld::new(
        &cli.font_path,
        package_dir,
        cli.package_path.clone(),
        cli.allow_download,
        cli.meter,
    );
    if !cli.meter {
        tracing::info!(fonts = font_count, "{}", i18n::log_fonts_loaded());
    }

    match &cli.command {
        Command::Worker { max_compilations } => {
            worker::run(&cli, *max_compilations, &mut world, font_count, &shutdown)
        }
        Command::Interactive {
            open,
            template,
            max_compilations,
        } => interactive::run(&cli, *open, *template, *max_compilations, &mut world, &shutdown),
        Command::Pipe { template } => pipe::run(&cli, *template, &mut world),
    }
}
