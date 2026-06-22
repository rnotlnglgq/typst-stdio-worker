/// Compilation and rendering of typst source to PNG.
use image::ImageEncoder;
use typst::World;
use typst::WorldExt;
use typst::diag::{Severity, SourceDiagnostic};
use typst::utils::Scalar;
use typst_layout::PagedDocument;
use typst_render::RenderOptions;

use crate::i18n;
use crate::world::TypstBotWorld;


impl TypstBotWorld {
    /// Compile typst source and render all pages as a single vertically-stitched PNG.
    ///
    /// `prelude_lines` is the number of source lines injected by the active template
    /// before user-supplied content; reported diagnostic line numbers are shifted
    /// back by this amount so they match the user's input.
    pub fn compile_and_render(
        &mut self,
        source: &str,
        scale: f32,
        max_pages: usize,
        max_pixels: u64,
        prelude_lines: usize,
        meter: bool,
    ) -> Result<RenderOk, Vec<CompileError>> {
        self.update_source(source.to_string());

        let warned = typst::compile::<PagedDocument>(self);

        let warnings: Vec<CompileError> = warned
            .warnings
            .iter()
            .map(|d| diagnostic_to_message(self, d, prelude_lines))
            .collect();

        for w in &warnings {
            tracing::warn!("{}", w.message);
        }

        let document = warned.output.map_err(|diagnostics| {
            diagnostics
                .iter()
                .map(|d| diagnostic_to_message(self, d, prelude_lines))
                .collect::<Vec<_>>()
        })?;

        if document.pages().is_empty() {
            return Err(vec![CompileError::limit(ErrorCode::NoPages)]);
        }

        let page_count = document.pages().len();
        if page_count > max_pages {
            return Err(vec![CompileError::limit(ErrorCode::TooManyPages {
                actual: page_count,
                limit: max_pages,
            })]);
        }

        let render_opts = RenderOptions {
            pixel_per_pt: Scalar::new(scale as f64),
            ..Default::default()
        };
        let pixmaps: Vec<_> = document
            .pages()
            .iter()
            .map(|page| typst_render::render(page, &render_opts))
            .collect();

        let total_width = pixmaps.iter().map(|p| p.width()).max().unwrap_or(0);
        let total_height: u32 = pixmaps.iter().map(|p| p.height()).sum();

        let total_pixels = total_width as u64 * total_height as u64;
        if total_pixels > max_pixels {
            return Err(vec![CompileError::limit(ErrorCode::TooManyPixels {
                width: total_width,
                height: total_height,
                pixels: total_pixels,
                limit: max_pixels,
            })]);
        }

        if total_width == 0 || total_height == 0 {
            return Err(vec![CompileError::limit(ErrorCode::ZeroSize)]);
        }

        let mut combined = vec![255u8; (total_pixels * 4) as usize];
        let mut y_offset: u32 = 0;
        for pixmap in &pixmaps {
            let pw = pixmap.width();
            let ph = pixmap.height();
            let data = pixmap.data();
            for row in 0..ph {
                let src_start = (row * pw * 4) as usize;
                let src_end = src_start + (pw * 4) as usize;
                let dst_start = ((y_offset + row) * total_width * 4) as usize;
                let dst_end = dst_start + (pw * 4) as usize;
                combined[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
            }
            y_offset += ph;
        }

        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(
                &combined,
                total_width,
                total_height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| {
                vec![CompileError::internal(ErrorCode::PngEncodingFailed {
                    detail: e.to_string(),
                })]
            })?;

        if meter {
            tracing::info!(
                pages = page_count,
                width = total_width,
                height = total_height,
                pixels = total_pixels,
                png_bytes = png_bytes.len() as u64,
                png_human = %crate::util::human_bytes::format_u64(png_bytes.len() as u64),
                "{}", i18n::log_render_complete()
            );
        }

        Ok(RenderOk {
            png: png_bytes,
            pages: page_count,
            warnings,
        })
    }
}

fn diagnostic_to_message(
    world: &TypstBotWorld,
    diag: &SourceDiagnostic,
    prelude_lines: usize,
) -> CompileError {
    let kind = match diag.severity {
        Severity::Error => ErrorKind::Compile,
        Severity::Warning => ErrorKind::Warning,
    };

    let raw_span = diag.span.id().and_then(|id| {
        let range = world.range(diag.span)?;
        let source = world.source(id).ok()?;
        let line = source.lines().byte_to_line(range.start)?;
        let column = source.lines().byte_to_column(range.start)?;
        Some((id == world.main_id(), line + 1, column + 1))
    });

    let (kind, span) = match raw_span {
        Some((true, line, column)) => {
            // Diagnostic in the main file. Subtract prelude lines so the user
            // sees positions relative to their own input.
            if line > prelude_lines {
                (
                    kind,
                    Some(ErrorSpan {
                        line: line - prelude_lines,
                        column,
                    }),
                )
            } else {
                // Diagnostic landed inside template-injected prelude. This is
                // a worker bug, not user input.
                (
                    ErrorKind::Internal,
                    Some(ErrorSpan { line, column }),
                )
            }
        }
        Some((false, line, column)) => {
            // Foreign file (e.g. package): report position as-is, no offset.
            (kind, Some(ErrorSpan { line, column }))
        }
        None => (kind, None),
    };

    let message = if kind == ErrorKind::Internal && span.is_some() {
        format!("{}: {}", i18n::log_template_internal_error(), diag.message)
    } else {
        diag.message.to_string()
    };

    let hints = diag.hints.iter().map(|h| h.v.to_string()).collect();

    CompileError {
        kind,
        message,
        span,
        hints,
        code: None,
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// A structured compile-side message: error or warning.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompileError {
    pub kind: ErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<ErrorSpan>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<String>,
    /// Structured error code for worker-generated errors. `None` for typst
    /// compiler diagnostics. Not serialized to the wire protocol.
    #[serde(skip)]
    pub code: Option<ErrorCode>,
}

/// Classification used by parent processes to route handling.
#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    /// Error originating in user typst source.
    Compile,
    /// Warning emitted by the typst compiler. Carried in `Response.warnings`.
    Warning,
    /// Resource limit hit (max_pages, max_pixels, empty document).
    Limit,
    /// Worker-internal error (encoding failure, unimplemented feature).
    Internal,
    /// Protocol-level error (bad JSON, oversized request).
    Protocol,
}

/// Location of a diagnostic in source.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorSpan {
    pub line: usize,
    pub column: usize,
}

/// Structured error codes for worker-generated errors.
///
/// Carries structured data so that error messages can be formatted in the
/// user's locale at display time while the wire protocol always receives a
/// stable English `message` string.
#[derive(Debug, Clone)]
pub enum ErrorCode {
    /// Document compiled but produced zero pages.
    NoPages,
    /// Document exceeded the page limit.
    TooManyPages { actual: usize, limit: usize },
    /// Rendered image exceeded the pixel limit.
    TooManyPixels { width: u32, height: u32, pixels: u64, limit: u64 },
    /// Rendered image has zero dimensions.
    ZeroSize,
    /// PNG encoding failed.
    PngEncodingFailed { detail: String },
    /// Feature not yet implemented.
    Unsupported { feature: String },
    /// Protocol-level problem (bad JSON, oversized request).
    ProtocolError { detail: String },
}

impl ErrorCode {
    /// Stable English message for the wire protocol.
    pub fn message_en(&self) -> String {
        match self {
            Self::NoPages => "document produced no pages".into(),
            Self::TooManyPages { actual, limit } => {
                format!("document has {} pages, exceeding limit of {}", actual, limit)
            }
            Self::TooManyPixels { width, height, pixels, limit } => {
                format!(
                    "rendered image {}x{} ({} pixels) exceeds limit of {} pixels",
                    width, height, pixels, limit
                )
            }
            Self::ZeroSize => "rendered image has zero size".into(),
            Self::PngEncodingFailed { detail } => format!("PNG encoding failed: {}", detail),
            Self::Unsupported { feature } => {
                format!("{} is not yet supported by this worker", feature)
            }
            Self::ProtocolError { detail } => detail.clone(),
        }
    }

    /// Localized message for CLI display (stderr).
    pub fn message_local(&self) -> String {
        match i18n::detect_lang() {
            i18n::Lang::En => self.message_en(),
            i18n::Lang::Zh => match self {
                Self::NoPages => "文档未生成任何页面".into(),
                Self::TooManyPages { actual, limit } => {
                    format!("文档有 {} 页，超过限制 {} 页", actual, limit)
                }
                Self::TooManyPixels { width, height, pixels, limit } => {
                    format!(
                        "渲染图像 {}x{}（{} 像素）超过限制 {} 像素",
                        width, height, pixels, limit
                    )
                }
                Self::ZeroSize => "渲染图像尺寸为零".into(),
                Self::PngEncodingFailed { detail } => format!("PNG 编码失败：{}", detail),
                Self::Unsupported { feature } => {
                    format!("此工作进程尚不支持 {}", feature)
                }
                Self::ProtocolError { detail } => detail.clone(),
            },
        }
    }
}

impl CompileError {
    /// Create from a structured error code. The `message` field is
    /// auto-populated with the stable English text.
    pub fn from_code(kind: ErrorKind, code: ErrorCode) -> Self {
        Self {
            kind,
            message: code.message_en(),
            span: None,
            hints: Vec::new(),
            code: Some(code),
        }
    }

    pub fn limit(code: ErrorCode) -> Self {
        Self::from_code(ErrorKind::Limit, code)
    }

    pub fn internal(code: ErrorCode) -> Self {
        Self::from_code(ErrorKind::Internal, code)
    }

    pub fn protocol(code: ErrorCode) -> Self {
        Self::from_code(ErrorKind::Protocol, code)
    }
}

/// Successful render result: PNG bytes, rendered page count, and any warnings
/// the compiler produced along the way.
pub struct RenderOk {
    pub png: Vec<u8>,
    pub pages: usize,
    pub warnings: Vec<CompileError>,
}
