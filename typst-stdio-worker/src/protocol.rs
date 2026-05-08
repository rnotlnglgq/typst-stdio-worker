/// NDJSON protocol for worker mode communication.
use std::io::{BufRead, Write};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::render::CompileError;
use crate::template::TemplateKind;

/// Protocol version reported in the ready message.
///
/// Bumped on any breaking change to request/response structure or semantics.
pub const PROTOCOL_VERSION: u32 = 1;

/// A compilation request from the parent process.
#[derive(Debug, Deserialize)]
pub struct Request {
    /// Optional correlation ID, echoed in response.
    #[serde(default)]
    pub id: Option<String>,
    /// Typst source code.
    pub source: String,
    /// Template to apply: "raw" or "default".
    #[serde(default)]
    pub template: TemplateStr,
    /// PNG pixel scale.
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Maximum number of pages to render. Falls back to CLI default when unset.
    #[serde(default)]
    pub max_pages: Option<usize>,
    /// Output format. Currently only `"png"` is supported.
    #[serde(default)]
    pub format: OutputFormat,
    /// Whether to vertically stitch all pages into a single image. Currently
    /// only `true` is supported; `false` is reserved for future expansion.
    #[serde(default = "default_stitch")]
    pub stitch: bool,
}

/// Template selection in the protocol (string-based for serde).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateStr {
    #[default]
    Raw,
    #[allow(dead_code)]
    DeprecatedDefault,
    Quiconf,
}

// This is not duplicate since it is possible to have a `TemplateKind` as an enum with variant
impl From<&TemplateStr> for TemplateKind {
    fn from(s: &TemplateStr) -> Self {
        match s {
            TemplateStr::Raw => TemplateKind::Raw,
            TemplateStr::DeprecatedDefault => TemplateKind::DeprecatedDefault,
            TemplateStr::Quiconf => TemplateKind::Quiconf,
        }
    }
}

/// Output format requested by the parent. Reserved for future expansion (PDF/SVG).
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Png,
}

const fn default_scale() -> f32 {
    4.0
}

const fn default_stitch() -> bool {
    true
}

/// A compilation response to the parent process.
#[derive(Debug, Serialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<CompileError>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<CompileError>>,
}

impl Response {
    pub fn success(
        id: Option<String>,
        format: OutputFormat,
        png_data: &[u8],
        pages: usize,
        warnings: Vec<CompileError>,
    ) -> Self {
        Self {
            id,
            ok: true,
            format: Some(format),
            data: Some(base64::engine::general_purpose::STANDARD.encode(png_data)),
            pages: Some(pages),
            errors: None,
            warnings: if warnings.is_empty() { None } else { Some(warnings) },
        }
    }

    pub fn failure(id: Option<String>, errors: Vec<CompileError>) -> Self {
        Self {
            id,
            ok: false,
            format: None,
            data: None,
            pages: None,
            errors: Some(errors),
            warnings: None,
        }
    }
}

/// Sent once after initialization to signal readiness.
#[derive(Debug, Serialize)]
pub struct ReadyMessage {
    pub ready: bool,
    pub protocol_version: u32,
    pub version: String,
    pub fonts_loaded: usize,
}

/// Outcome of a single line read in [`read_request`].
enum LineRead {
    Eof,
    Empty,
    Line(String),
    Oversized,
}

/// Read one line, capping the bytes pulled from `reader` at `max_input_size + 1`
/// so a malicious peer cannot exhaust memory by sending an unbounded chunk
/// without a newline. If the cap is hit we drain the rest of the line and
/// return [`LineRead::Oversized`].
fn read_capped_line(reader: &mut impl BufRead, max_input_size: usize) -> std::io::Result<LineRead> {
    let mut buf = Vec::with_capacity(256.min(max_input_size + 1));
    let cap = max_input_size as u64 + 1;
    let mut limited = std::io::Read::take(&mut *reader, cap);
    // Take<&mut R> still implements BufRead via the inner reader's buffer.
    let n = std::io::BufRead::read_until(&mut limited, b'\n', &mut buf)?;

    if n == 0 {
        return Ok(LineRead::Eof);
    }

    let oversized = buf.len() > max_input_size
        || (buf.len() as u64 == cap && !buf.ends_with(b"\n"));

    if oversized {
        // Drain to next newline so the next read starts on a clean boundary.
        let mut sink = Vec::new();
        loop {
            sink.clear();
            let m = reader.read_until(b'\n', &mut sink)?;
            if m == 0 || sink.ends_with(b"\n") {
                break;
            }
        }
        return Ok(LineRead::Oversized);
    }

    let s = String::from_utf8(buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        Ok(LineRead::Empty)
    } else {
        Ok(LineRead::Line(s))
    }
}

/// Outcome returned to the worker loop.
pub enum ReadOutcome {
    /// stdin closed, worker should shut down.
    Eof,
    /// A valid request.
    Request(Request),
    /// Recoverable error (bad JSON, oversized line). Worker should reply with
    /// a Protocol error and continue.
    Protocol(String),
}

/// Read one request from `reader`, skipping blank lines.
pub fn read_request(reader: &mut impl BufRead, max_input_size: usize) -> ReadOutcome {
    loop {
        match read_capped_line(reader, max_input_size) {
            Ok(LineRead::Eof) => return ReadOutcome::Eof,
            Ok(LineRead::Empty) => continue,
            Ok(LineRead::Oversized) => {
                return ReadOutcome::Protocol(format!(
                    "request too large (limit: {} bytes)",
                    max_input_size
                ));
            }
            Ok(LineRead::Line(line)) => {
                return match serde_json::from_str(line.trim()) {
                    Ok(req) => ReadOutcome::Request(req),
                    Err(e) => ReadOutcome::Protocol(format!("invalid JSON: {}", e)),
                };
            }
            Err(e) => {
                return ReadOutcome::Protocol(format!("failed to read stdin: {}", e));
            }
        }
    }
}

/// Write a response as a single JSON line to stdout.
pub fn write_response(writer: &mut impl Write, response: &Response) -> Result<(), String> {
    let json = serde_json::to_string(response).map_err(|e| format!("serialize error: {}", e))?;
    writeln!(writer, "{}", json).map_err(|e| format!("write error: {}", e))?;
    writer.flush().map_err(|e| format!("flush error: {}", e))
}

/// Write the ready message.
pub fn write_ready(writer: &mut impl Write, msg: &ReadyMessage) -> Result<(), String> {
    let json = serde_json::to_string(msg).map_err(|e| format!("serialize error: {}", e))?;
    writeln!(writer, "{}", json).map_err(|e| format!("write error: {}", e))?;
    writer.flush().map_err(|e| format!("flush error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    use crate::render::ErrorCode;

    // -- read_request --

    #[test]
    fn read_request_valid_json() {
        let input = b"{\"source\":\"Hello\"}\n";
        let mut reader = BufReader::new(Cursor::new(input));
        match read_request(&mut reader, 4096) {
            ReadOutcome::Request(req) => {
                assert_eq!(req.source, "Hello");
            }
            other => panic!("expected Request, got {:?}", outcome_tag(&other)),
        }
    }

    #[test]
    fn read_request_eof() {
        let mut reader = BufReader::new(Cursor::new(b"" as &[u8]));
        assert!(matches!(read_request(&mut reader, 4096), ReadOutcome::Eof));
    }

    #[test]
    fn read_request_skips_blank_lines() {
        let input = b"\n\n   \n{\"source\":\"ok\"}\n";
        let mut reader = BufReader::new(Cursor::new(input));
        match read_request(&mut reader, 4096) {
            ReadOutcome::Request(req) => assert_eq!(req.source, "ok"),
            other => panic!("expected Request, got {:?}", outcome_tag(&other)),
        }
    }

    #[test]
    fn read_request_invalid_json() {
        let input = b"not json at all\n";
        let mut reader = BufReader::new(Cursor::new(input));
        match read_request(&mut reader, 4096) {
            ReadOutcome::Protocol(msg) => {
                assert!(msg.contains("invalid JSON"), "msg: {msg}");
            }
            other => panic!("expected Protocol, got {:?}", outcome_tag(&other)),
        }
    }

    #[test]
    fn read_request_oversized_then_recover() {
        let big = "a".repeat(200);
        let mut input = format!("{{\"source\":\"{big}\"}}\n");
        input.push_str("{\"source\":\"small\"}\n");
        let mut reader = BufReader::new(Cursor::new(input.into_bytes()));

        // First read: oversized (limit=64).
        match read_request(&mut reader, 64) {
            ReadOutcome::Protocol(msg) => {
                assert!(msg.contains("too large"), "msg: {msg}");
            }
            other => panic!("expected Protocol, got {:?}", outcome_tag(&other)),
        }

        // Second read: normal request succeeds.
        match read_request(&mut reader, 4096) {
            ReadOutcome::Request(req) => assert_eq!(req.source, "small"),
            other => panic!("expected Request, got {:?}", outcome_tag(&other)),
        }
    }

    #[test]
    fn read_request_non_utf8() {
        let input: &[u8] = &[0x80, 0x81, 0x82, b'\n'];
        let mut reader = BufReader::new(Cursor::new(input));
        match read_request(&mut reader, 4096) {
            ReadOutcome::Protocol(msg) => {
                assert!(msg.contains("read stdin"), "msg: {msg}");
            }
            other => panic!("expected Protocol, got {:?}", outcome_tag(&other)),
        }
    }

    // -- Request deserialization defaults --

    #[test]
    fn request_defaults() {
        let json = r#"{"source":"x"}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert_eq!(req.source, "x");
        assert!(req.id.is_none());
        assert!(matches!(req.template, TemplateStr::Raw));
        assert_eq!(req.scale, 4.0);
        assert!(req.max_pages.is_none());
        assert_eq!(req.format, OutputFormat::Png);
        assert!(req.stitch);
    }

    // -- write_response --

    #[test]
    fn write_response_success_format() {
        let resp = Response::success(
            Some("req-1".into()),
            OutputFormat::Png,
            b"fakepng",
            3,
            Vec::new(),
        );
        let mut buf = Vec::new();
        write_response(&mut buf, &resp).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["id"], "req-1");
        assert_eq!(v["format"], "png");
        assert_eq!(v["pages"], 3);
        assert!(v["data"].as_str().unwrap().len() > 0);
        assert!(v.get("errors").is_none() || v["errors"].is_null());
        assert!(v.get("warnings").is_none() || v["warnings"].is_null());
    }

    #[test]
    fn write_response_failure_format() {
        let resp = Response::failure(
            None,
            vec![CompileError::limit(ErrorCode::TooManyPages {
                actual: 3,
                limit: 2,
            })],
        );
        let mut buf = Vec::new();
        write_response(&mut buf, &resp).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v.get("id").is_none() || v["id"].is_null());
        assert!(v.get("data").is_none() || v["data"].is_null());
        assert!(v.get("format").is_none() || v["format"].is_null());
        assert!(v.get("pages").is_none() || v["pages"].is_null());
        let errors = v["errors"].as_array().unwrap();
        assert_eq!(errors[0]["kind"], "limit");
        assert_eq!(
            errors[0]["message"],
            "document has 3 pages, exceeding limit of 2"
        );
    }

    // -- write_ready --

    #[test]
    fn write_ready_format() {
        let msg = ReadyMessage {
            ready: true,
            protocol_version: PROTOCOL_VERSION,
            version: "0.1.0".into(),
            fonts_loaded: 42,
        };
        let mut buf = Vec::new();
        write_ready(&mut buf, &msg).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(v["ready"], true);
        assert_eq!(v["protocol_version"], 1);
        assert_eq!(v["version"], "0.1.0");
        assert_eq!(v["fonts_loaded"], 42);
    }

    fn outcome_tag(o: &ReadOutcome) -> &'static str {
        match o {
            ReadOutcome::Eof => "Eof",
            ReadOutcome::Request(_) => "Request",
            ReadOutcome::Protocol(_) => "Protocol",
        }
    }
}
