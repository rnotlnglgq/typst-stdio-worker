//! Typst compilation worker binary. Dispatches to [`cli::run`].
//!
//! **Modules** (rough dependency order: lower layers first, CLI last):
//! - [`i18n`] — localized help strings and log messages
//! - [`util`] — small helpers
//! - [`template`] — prelude / template wrapping
//! - [`protocol`] — NDJSON worker wire format
//! - [`world`] — Typst world: fonts, packages, virtual files
//! - [`render`] — compile source and encode PNG
//! - [`cli`] — argument parsing and mode handlers

mod i18n;
mod prelude_loader;
mod protocol;
mod render;
mod template;
mod world;
mod util;
mod cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
