use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::protocol::{self, OutputFormat, PROTOCOL_VERSION, ReadOutcome, ReadyMessage, Response};
use crate::render::{CompileError, ErrorCode, RenderOk};
use crate::template::TemplateKind;
use crate::world::TypstBotWorld;

use super::Cli;
use crate::i18n;

pub fn run(
    cli: &Cli,
    max_compilations: u64,
    world: &mut TypstBotWorld,
    font_count: usize,
    shutdown: &AtomicBool,
) -> ExitCode {
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();
    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();

    let ready = ReadyMessage {
        ready: true,
        protocol_version: PROTOCOL_VERSION,
        version: env!("CARGO_PKG_VERSION").to_string(),
        fonts_loaded: font_count,
    };

    if let Err(e) = protocol::write_ready(&mut stdout_lock, &ready) {
        tracing::error!(error = %e, "{}", i18n::log_write_ready_failed());
        return ExitCode::FAILURE;
    }

    let mut compilation_count: u64 = 0;

    loop {
        if shutdown.load(Ordering::Relaxed) {
            tracing::info!("{}", i18n::log_shutdown_signal());
            break;
        }

        let request = match protocol::read_request(&mut stdin_lock, cli.max_input_size) {
            ReadOutcome::Eof => {
                tracing::info!("{}", i18n::log_stdin_closed());
                break;
            }
            ReadOutcome::Request(req) => req,
            ReadOutcome::Protocol(msg) => {
                tracing::warn!(detail = %msg, "{}", i18n::log_protocol_error());
                let resp = Response::failure(
                    None,
                    vec![CompileError::protocol(ErrorCode::ProtocolError {
                        detail: msg,
                    })],
                );
                if let Err(e) = protocol::write_response(&mut stdout_lock, &resp) {
                    tracing::error!(error = %e, "{}", i18n::log_write_response_failed());
                    break;
                }
                continue;
            }
        };

        if !request.stitch {
            let resp = Response::failure(
                request.id,
                vec![CompileError::internal(ErrorCode::Unsupported {
                    feature: "stitch=false".into(),
                })],
            );
            if let Err(e) = protocol::write_response(&mut stdout_lock, &resp) {
                tracing::error!(error = %e, "{}", i18n::log_write_response_failed());
                break;
            }
            continue;
        }

        let template_kind = TemplateKind::from(&request.template);
        let max_pages = request.max_pages.unwrap_or(cli.max_pages);
        let scale = request.scale;
        let format = request.format;
        let (wrapped, prelude_lines) = template_kind.apply_to(&request.source);

        let response = match world.compile_and_render(
            &wrapped,
            scale,
            max_pages,
            cli.max_pixels,
            prelude_lines,
            cli.meter,
        ) {
            Ok(RenderOk { png, pages, warnings }) => {
                Response::success(request.id, format, &png, pages, warnings)
            }
            Err(errors) => Response::failure(request.id, errors),
        };

        if let Err(e) = protocol::write_response(&mut stdout_lock, &response) {
            tracing::error!(error = %e, "{}", i18n::log_write_response_failed());
            break;
        }

        comemo::evict(5);
        compilation_count += 1;

        if max_compilations > 0 && compilation_count >= max_compilations {
            tracing::info!(
                count = compilation_count,
                "{}", i18n::log_max_compilations()
            );
            break;
        }
    }

    let _ = OutputFormat::Png;
    ExitCode::SUCCESS
}
