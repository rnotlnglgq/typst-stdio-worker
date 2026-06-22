use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Path to the binary built by cargo for this crate.
fn worker_binary() -> &'static str {
    env!("CARGO_BIN_EXE_typst-stdio-worker")
}

#[test]
fn pipe_mode_produces_valid_png() {
    let output = Command::new(worker_binary())
        .args(["--log-level", "error", "pipe", "--template", "raw"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"Hello, World!")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(output.status.success(), "worker exited with error: {}", String::from_utf8_lossy(&output.stderr));
    let png = &output.stdout;
    assert!(png.len() > 8, "output too small to be a PNG");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "not a valid PNG");
}

#[test]
fn pipe_mode_error_on_invalid_source() {
    let output = Command::new(worker_binary())
        .args(["--log-level", "error", "pipe", "--template", "raw"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"#unknownfunc()")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(!output.status.success(), "worker should fail on invalid source");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown variable"),
        "stderr should contain error message, got: {}",
        stderr
    );
}

#[test]
fn pipe_mode_sandbox_blocks_file_read() {
    let output = Command::new(worker_binary())
        .args(["--log-level", "error", "pipe", "--template", "raw"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"#read(\"/etc/passwd\")")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(!output.status.success(), "worker should fail on file read");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found"),
        "stderr should mention file not found, got: {}",
        stderr
    );
}

#[test]
fn pipe_mode_max_pages_limit() {
    let output = Command::new(worker_binary())
        .args([
            "--max-pages", "2",
            "--log-level", "error",
            "pipe", "--template", "raw",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"#set page(width: 100pt, height: 50pt)\n#lorem(500)")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(
        !output.status.success(),
        "worker should fail when exceeding max pages"
    );
}

#[test]
fn worker_mode_ready_and_request_response() {
    let mut child = Command::new(worker_binary())
        .args(["--log-level", "error", "worker"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Read ready message — must include protocol_version.
    let mut ready_line = String::new();
    reader.read_line(&mut ready_line).unwrap();
    let ready: serde_json::Value = serde_json::from_str(ready_line.trim()).unwrap();
    assert_eq!(ready["ready"], true);
    assert_eq!(ready["protocol_version"], 1);
    assert!(ready["fonts_loaded"].as_u64().unwrap() > 0);

    writeln!(stdin, r##"{{"source":"Hello"}}"##).unwrap();
    stdin.flush().unwrap();

    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(resp_line.trim()).unwrap();
    assert_eq!(resp["ok"], true);
    assert_eq!(resp["format"], "png");
    assert!(resp["data"].as_str().unwrap().len() > 10);
    assert!(resp["pages"].as_u64().unwrap() >= 1);

    let err_req = r##"{"source":"#badcall()","template":"raw"}"##;
    writeln!(stdin, "{}", err_req).unwrap();
    stdin.flush().unwrap();

    let mut err_line = String::new();
    reader.read_line(&mut err_line).unwrap();
    let err_resp: serde_json::Value = serde_json::from_str(err_line.trim()).unwrap();
    assert_eq!(err_resp["ok"], false);
    let errors = err_resp["errors"].as_array().unwrap();
    assert!(!errors.is_empty());
    assert_eq!(errors[0]["kind"], "compile");

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn worker_mode_max_compilations() {
    let mut child = Command::new(worker_binary())
        .args([
            "--log-level", "error",
            "worker", "--max-compilations", "2",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    for _ in 0..2 {
        writeln!(stdin, r##"{{"source":"Hi","template":"raw"}}"##).unwrap();
        stdin.flush().unwrap();
        let mut resp = String::new();
        reader.read_line(&mut resp).unwrap();
    }

    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn worker_skips_blank_lines() {
    let mut child = Command::new(worker_binary())
        .args(["--log-level", "error", "worker"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut ready_line = String::new();
    reader.read_line(&mut ready_line).unwrap();

    // Send several blank lines, then a real request. Worker must respond
    // exactly once and not treat the blanks as EOF.
    stdin.write_all(b"\n\n   \n").unwrap();
    writeln!(stdin, r##"{{"source":"Hi","template":"raw"}}"##).unwrap();
    stdin.flush().unwrap();

    let mut resp = String::new();
    reader.read_line(&mut resp).unwrap();
    let v: serde_json::Value = serde_json::from_str(resp.trim()).unwrap();
    assert_eq!(v["ok"], true, "response after blank lines: {}", resp);

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn worker_rejects_oversized_input() {
    let mut child = Command::new(worker_binary())
        .args([
            "--max-input-size", "128",
            "--log-level", "error",
            "worker",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut ready_line = String::new();
    reader.read_line(&mut ready_line).unwrap();

    // Build a 1024-byte JSON line — well over the 128 cap.
    let big_source = "a".repeat(900);
    let line = format!(r##"{{"source":"{}","template":"raw"}}"##, big_source);
    assert!(line.len() > 128);
    writeln!(stdin, "{}", line).unwrap();
    stdin.flush().unwrap();

    let mut resp = String::new();
    reader.read_line(&mut resp).unwrap();
    let v: serde_json::Value = serde_json::from_str(resp.trim()).unwrap();
    assert_eq!(v["ok"], false);
    let errors = v["errors"].as_array().unwrap();
    assert_eq!(errors[0]["kind"], "protocol", "got: {}", resp);

    // After draining the oversized line, a normal request must still work.
    writeln!(stdin, r##"{{"source":"Hi","template":"raw"}}"##).unwrap();
    stdin.flush().unwrap();
    let mut resp2 = String::new();
    reader.read_line(&mut resp2).unwrap();
    let v2: serde_json::Value = serde_json::from_str(resp2.trim()).unwrap();
    assert_eq!(v2["ok"], true, "follow-up response: {}", resp2);

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn worker_enforces_pixel_limit() {
    let mut child = Command::new(worker_binary())
        .args([
            "--max-pixels", "1000000",
            "--log-level", "error",
            "worker",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut ready_line = String::new();
    reader.read_line(&mut ready_line).unwrap();

    // Force a large page. 2000pt at scale 2.0 -> ~5333 px on a side.
    let req = r##"{"source":"#set page(width: 2000pt, height: 2000pt)\nbig","template":"raw","scale":2.0}"##;
    writeln!(stdin, "{}", req).unwrap();
    stdin.flush().unwrap();

    let mut resp = String::new();
    reader.read_line(&mut resp).unwrap();
    let v: serde_json::Value = serde_json::from_str(resp.trim()).unwrap();
    assert_eq!(v["ok"], false, "response: {}", resp);
    let errors = v["errors"].as_array().unwrap();
    assert_eq!(errors[0]["kind"], "limit");
    assert!(
        errors[0]["message"]
            .as_str()
            .unwrap()
            .contains("pixels"),
        "message: {}",
        errors[0]["message"]
    );

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn worker_returns_warnings_with_success() {
    let mut child = Command::new(worker_binary())
        .args(["--log-level", "error", "worker"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut ready_line = String::new();
    reader.read_line(&mut ready_line).unwrap();

    // Typst 0.15 deprecates zero in numbering systems that cannot represent it.
    let req = r##"{"source":"#numbering(\"harazi\", 0)\nhello","template":"raw"}"##;
    writeln!(stdin, "{}", req).unwrap();
    stdin.flush().unwrap();

    let mut resp = String::new();
    reader.read_line(&mut resp).unwrap();
    let v: serde_json::Value = serde_json::from_str(resp.trim()).unwrap();
    assert_eq!(v["ok"], true, "response: {}", resp);
    let warnings = v["warnings"].as_array().expect("warnings field present");
    assert!(!warnings.is_empty(), "expected at least one warning");
    assert_eq!(warnings[0]["kind"], "warning");

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

/// Compile a physica-using source in pipe mode, downloading the package into an
/// isolated temp directory so the test is hermetic.
///
/// Requires network access; run with `cargo test -- --ignored`.
#[test]
#[ignore]
fn pipe_mode_download_package_and_compile() {
    let pkg_dir = tempfile::tempdir().expect("failed to create temp dir");
    let source = r#"#import "@preview/physica:0.9.8": *
$ curl(grad f), tensor(T, -mu, +nu), pdv(f,x,y,[1,2]) $
"#;
    let output = Command::new(worker_binary())
        .args([
            "--allow-download",
            "--package-cache-path", pkg_dir.path().to_str().unwrap(),
            "--log-level", "error",
            "pipe", "--template", "raw",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(source.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "worker failed (package download+compile): {stderr}",
    );
    let png = &output.stdout;
    assert!(png.len() > 8, "output too small to be a PNG");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "not a valid PNG");
}

/// Layout: `{base}/local/{name}/0.1.0/{typst.toml, lib.typ}` for `@local/{name}:0.1.0`.
fn write_minimal_local_package(base: &Path, name: &str) {
    let dir = base.join("local").join(name).join("0.1.0");
    fs::create_dir_all(&dir).expect("mkdir package version");
    fs::write(
        dir.join("typst.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n"
        ),
    )
    .expect("write typst.toml");
    fs::write(dir.join("lib.typ"), "#let x = 1\n").expect("write lib.typ");
}

#[test]
fn pipe_mode_local_package_via_package_path_cli() {
    let base = tempfile::tempdir().expect("temp dir");
    write_minimal_local_package(base.path(), "tsw_loc");
    let source = "#import \"@local/tsw_loc:0.1.0\": x\n#str(x)\n";
    let output = Command::new(worker_binary())
        .args([
            "--log-level",
            "error",
            "--package-path",
            base.path().to_str().unwrap(),
            "pipe",
            "--template",
            "raw",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(&output.stdout[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn pipe_mode_local_package_via_package_path_env() {
    let base = tempfile::tempdir().expect("temp dir");
    write_minimal_local_package(base.path(), "tsw_loc_env");
    let source = "#import \"@local/tsw_loc_env:0.1.0\": x\n#str(x)\n";
    let output = Command::new(worker_binary())
        .env(
            "TYPST_PACKAGE_PATH",
            base.path().to_str().expect("utf8 path"),
        )
        .args(["--log-level", "error", "pipe", "--template", "raw"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(&output.stdout[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn pipe_mode_package_path_cli_overrides_env() {
    let wrong = tempfile::tempdir().expect("temp dir");
    let good = tempfile::tempdir().expect("temp dir");
    write_minimal_local_package(good.path(), "tsw_loc_cli");
    let source = "#import \"@local/tsw_loc_cli:0.1.0\": x\n#str(x)\n";
    let output = Command::new(worker_binary())
        .env(
            "TYPST_PACKAGE_PATH",
            wrong.path().to_str().expect("utf8 path"),
        )
        .args([
            "--log-level",
            "error",
            "--package-path",
            good.path().to_str().unwrap(),
            "pipe",
            "--template",
            "raw",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(&output.stdout[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn worker_echoes_request_id() {
    let mut child = Command::new(worker_binary())
        .args(["--log-level", "error", "worker"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut ready_line = String::new();
    reader.read_line(&mut ready_line).unwrap();

    // Request with an explicit id.
    writeln!(stdin, r#"{{"source":"Hi","template":"raw","id":"test-42"}}"#).unwrap();
    stdin.flush().unwrap();

    let mut resp = String::new();
    reader.read_line(&mut resp).unwrap();
    let v: serde_json::Value = serde_json::from_str(resp.trim()).unwrap();
    assert_eq!(v["ok"], true, "response: {}", resp);
    assert_eq!(v["id"], "test-42", "id must be echoed back");

    // Request without id — response must not contain the field.
    writeln!(stdin, r#"{{"source":"Hi","template":"raw"}}"#).unwrap();
    stdin.flush().unwrap();

    let mut resp2 = String::new();
    reader.read_line(&mut resp2).unwrap();
    let v2: serde_json::Value = serde_json::from_str(resp2.trim()).unwrap();
    assert_eq!(v2["ok"], true);
    assert!(v2.get("id").is_none() || v2["id"].is_null(), "id should be absent");

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn worker_rejects_stitch_false() {
    let mut child = Command::new(worker_binary())
        .args(["--log-level", "error", "worker"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn worker");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut ready_line = String::new();
    reader.read_line(&mut ready_line).unwrap();

    writeln!(
        stdin,
        r#"{{"source":"Hi","template":"raw","stitch":false}}"#
    )
    .unwrap();
    stdin.flush().unwrap();

    let mut resp = String::new();
    reader.read_line(&mut resp).unwrap();
    let v: serde_json::Value = serde_json::from_str(resp.trim()).unwrap();
    assert_eq!(v["ok"], false, "stitch=false should be rejected: {}", resp);
    let errors = v["errors"].as_array().unwrap();
    assert_eq!(errors[0]["kind"], "internal");

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn pipe_mode_default_template() {
    let output = Command::new(worker_binary())
        .args(["--log-level", "error", "pipe"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"Hello, World!")
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run worker");

    assert!(
        output.status.success(),
        "pipe default template failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let png = &output.stdout;
    assert!(png.len() > 8, "output too small to be a PNG");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "not a valid PNG");
}
