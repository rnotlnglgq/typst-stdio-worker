use std::io::{BufRead, Write};
use std::process::{ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::render::RenderOk;
use crate::template::TemplateKind;
use crate::world::TypstBotWorld;

use super::Cli;
use crate::i18n;
use super::pipe::print_diag_to_stderr;

pub fn run(
    cli: &Cli,
    open: bool,
    template_kind: TemplateKind,
    max_compilations: u64,
    world: &mut TypstBotWorld,
    shutdown: &AtomicBool,
) -> ExitCode {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();

    let tmp_path = std::env::temp_dir().join("typst-stdio-worker-preview.png");

    let mut compilation_count: u64 = 0;

    eprintln!("{}", i18n::msg_interactive_banner());
    eprintln!("{} {}", i18n::msg_output_path(), tmp_path.display());
    if open {
        eprintln!("{}", i18n::msg_open_hint());
    }
    eprintln!("{}", i18n::msg_exit_hint());
    eprint!("> ");
    std::io::stderr().flush().ok();

    loop {
        if shutdown.load(Ordering::Relaxed) {
            eprintln!("\n{}", i18n::msg_shutting_down());
            break;
        }

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                eprintln!("\n{}", i18n::msg_eof_exit());
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", i18n::fmt_error(i18n::err_read_line(), &e));
                break;
            }
        }

        let input = line.trim();
        if input.is_empty() {
            eprint!("> ");
            std::io::stderr().flush().ok();
            continue;
        }

        if input.len() > cli.max_input_size {
            eprintln!(
                "{}",
                i18n::fmt_input_too_large(input.len(), cli.max_input_size)
            );
            eprint!("> ");
            std::io::stderr().flush().ok();
            continue;
        }

        let (wrapped, prelude_lines) = template_kind.apply_to(input);

        match world.compile_and_render(
            &wrapped,
            cli.scale,
            cli.max_pages,
            cli.max_pixels,
            prelude_lines,
            cli.meter,
        ) {
            Ok(RenderOk { png, pages, warnings }) => {
                for w in &warnings {
                    print_diag_to_stderr(w);
                }
                match std::fs::write(&tmp_path, &png) {
                    Ok(_) => {
                        eprintln!("{}", i18n::format_compile_ok(pages, png.len(), &tmp_path));
                        if open {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(&tmp_path)
                                .stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn();
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", i18n::fmt_error(i18n::err_write_file(), &e));
                    }
                }
            }
            Err(errors) => {
                for error in &errors {
                    print_diag_to_stderr(error);
                }
            }
        }

        comemo::evict(5);
        compilation_count += 1;

        if max_compilations > 0 && compilation_count >= max_compilations {
            eprintln!(
                "{} ({})",
                i18n::msg_max_compilations_reached(),
                compilation_count
            );
            break;
        }

        eprint!("> ");
        std::io::stderr().flush().ok();
    }

    ExitCode::SUCCESS
}
