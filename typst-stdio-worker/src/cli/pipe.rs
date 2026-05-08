use std::io::{Read, Write};
use std::process::ExitCode;

use crate::render::{CompileError, ErrorKind, RenderOk};
use crate::template::TemplateKind;
use crate::world::TypstBotWorld;

use super::Cli;
use crate::i18n;

pub fn run(cli: &Cli, template_kind: TemplateKind, world: &mut TypstBotWorld) -> ExitCode {
    let mut source = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("{}", i18n::fmt_error(i18n::err_read_stdin(), &e));
        return ExitCode::FAILURE;
    }

    if source.len() > cli.max_input_size {
        eprintln!("{}", i18n::fmt_input_too_large(source.len(), cli.max_input_size));
        return ExitCode::FAILURE;
    }

    let (wrapped, prelude_lines) = template_kind.apply_to(&source);

    match world.compile_and_render(
        &wrapped,
        cli.scale,
        cli.max_pages,
        cli.max_pixels,
        prelude_lines,
        cli.meter,
    ) {
        Ok(RenderOk { png, pages: _, warnings }) => {
            for w in &warnings {
                print_diag_to_stderr(w);
            }
            if let Err(e) = std::io::stdout().write_all(&png) {
                eprintln!("{}", i18n::fmt_error(i18n::err_write_output(), &e));
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in &errors {
                print_diag_to_stderr(error);
            }
            ExitCode::FAILURE
        }
    }
}

pub fn print_diag_to_stderr(diag: &CompileError) {
    let label = match diag.kind {
        ErrorKind::Warning => i18n::dlg_label_warning(),
        _ => i18n::dlg_label_error(),
    };
    let message = match &diag.code {
        Some(code) => code.message_local(),
        None => diag.message.clone(),
    };
    if let Some(ref span) = diag.span {
        eprintln!("{}:{}:{}: {}", label, span.line, span.column, message);
    } else {
        eprintln!("{}: {}", label, message);
    }
    for hint in &diag.hints {
        eprintln!("{}{}", i18n::dlg_hint_prefix(), hint);
    }
}
