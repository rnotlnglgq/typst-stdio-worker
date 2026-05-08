# Typst Stdio Worker

Language: **English** · [简体中文](README.zh-CN.md)

This project is built on the Rust `typst` crate. It accepts typesetting jobs over STDIO using an NDJSON protocol and returns results with BASE64-encoded payloads inside JSON. The main goal is to isolate this process so callers can apply OS-level quotas and resource limits.

## NDJSON protocol (`worker`)

Communication is **newline-delimited JSON** (one JSON object per line). After fonts load, the worker prints a **ready** line on stdout:

```json
{"ready":true,"protocol_version":1,"version":"0.1.0","fonts_loaded":2464}
```

The parent writes **requests** to stdin (one JSON per line). Required: `source`. Optional: `id`, `template` (`raw` / `default`), `scale`, `max_pages`, `format` (`png`), `stitch`, and other fields—see [docs/NOTES.md](docs/NOTES.md).

Example request:

```json
{"id":"req-1","source":"Hello","template":"default","scale":2.0,"max_pages":50,"format":"png","stitch":true}
```

Each **response** is one JSON line on stdout. Success (optional `id`, `warnings`):

```json
{"id":"req-1","ok":true,"format":"png","data":"<base64 PNG>","pages":3}
```

Failure (`errors[]` holds `kind`, optional `span`, `hints`):

```json
{"id":"req-1","ok":false,"errors":[{"kind":"compile","message":"…","span":{"line":3,"column":5},"hints":[]}]}
```

Logs go to stderr only.

## Implementation Overview

It implements the `typst::World` trait with the caching behavior Typst expects, plus logging with basic i18n support. Template handling is still evolving; it currently relies heavily on string concatenation, with more logic delegated to local Typst packages. Network download of Typst packages is disabled by default; use `--allow-download` to enable it. A typical deployment pattern is for the operator to warm the cache for `@preview/` packages with `--allow-download` enabled, then serve users with downloads turned off.

Fonts are read via `unsafe` `memmap2` so multiple concurrent instances can share memory-mapped font data. Prefer dedicated font copies and Linux ACLs so font files are not modified accidentally.

## Environment variables

| Variable | Effect |
| --- | --- |
| `TSW_LANG` | Force CLI/help language: values such as `zh`, `zh_cn`, `zh_tw`, or `chinese` select Chinese; anything else selects English. Overrides `LANG`. |
| `LANG` | If `TSW_LANG` is unset, messages use Chinese when `LANG` starts with `zh`, otherwise English. |
| `TSW_RESOURCES` | Directory for bundled text assets (prelude snippets under `resources/`). Absolute path, or relative to the process working directory. |
| `TYPST_PACKAGE_PATH` | Default for `--package-path` (local `@local/` packages). CLI wins if both are set. |
| `TYPST_PACKAGE_CACHE_PATH` | Default for `--package-cache-path` (registry cache). CLI wins if both are set. |
| `XDG_CACHE_HOME` | When neither `--package-cache-path` nor `TYPST_PACKAGE_CACHE_PATH` is set, the cache falls back to `$XDG_CACHE_HOME/typst/packages`. |
| `HOME` | If `XDG_CACHE_HOME` is also unset, cache falls back to `$HOME/.cache/typst/packages`. |
| `RUST_LOG` | If set to a valid `tracing-subscriber` filter string, it overrides the default from `--log-level` (see `EnvFilter` in the tracing ecosystem). |

The OS temporary directory (e.g. for interactive preview PNGs) follows the usual platform rules (`TMPDIR` on Unix, etc.).

## CLI reference

```
Typst compilation worker — renders Typst source to PNG via pipe, interactive REPL, or long-running NDJSON protocol.

Usage: typst-stdio-worker [OPTIONS] <COMMAND>

Commands:
  worker       Long-running NDJSON worker (stdin/stdout protocol)
  interactive  Interactive REPL: type source, press Enter to compile and preview
  pipe         One-shot pipe mode: read stdin, compile, write PNG to stdout
  help         Print this message or the help of the given subcommand(s)

Options:
  -f, --font-path <FONT_PATH>
          Additional font directories to load
  -l, --package-path <DIR>
          Custom path to local packages (TYPST_PACKAGE_PATH; same as typst compile --package-path). CLI overrides env. Resolved before the package cache root. [env: TYPST_PACKAGE_PATH=]
  -p, --package-cache-path <DIR>
          Custom path to package cache (TYPST_PACKAGE_CACHE_PATH; same as typst compile --package-cache-path). Falls back to $XDG_CACHE_HOME/typst/packages or ~/.cache/typst/packages when unset. [env: TYPST_PACKAGE_CACHE_PATH=]
      --allow-download
          Allow downloading missing packages from the Typst package registry
  -s, --scale <SCALE>
          PNG pixel scale factor [default: 4.0]
      --max-pages <MAX_PAGES>
          Maximum number of pages to render [default: 1]
      --max-pixels <MAX_PIXELS>
          Maximum total pixels (width*height across all pages) [default: 100000000]
      --max-input-size <MAX_INPUT_SIZE>
          Maximum input size in bytes [default: 1048576]
      --log-level <LOG_LEVEL>
          Log level filter [default: info]
      --meter
          Log aggregate font file bytes (deduplicated) and each rendered PNG size / dimensions
  -h, --help
          Print help
  -V, --version
          Print version
```
