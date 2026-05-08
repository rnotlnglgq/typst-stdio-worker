//! Bilingual (Chinese/English) text support.
//!
//! Language detection priority:
//! 1. `TSW_LANG` environment variable (explicit override)
//! 2. Standard `LANG` environment variable (system locale)
//! 3. Default to English

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

/// Detect the user's preferred language from environment variables.
pub fn detect_lang() -> Lang {
    if let Ok(v) = std::env::var("TSW_LANG") {
        return match v.to_lowercase().as_str() {
            "zh" | "zh_cn" | "zh_tw" | "chinese" => Lang::Zh,
            _ => Lang::En,
        };
    }
    if let Ok(v) = std::env::var("LANG") {
        if v.starts_with("zh") {
            return Lang::Zh;
        }
    }
    Lang::En
}

macro_rules! bilingual {
    ($name:ident, $en:expr, $zh:expr) => {
        pub fn $name() -> &'static str {
            match $crate::i18n::detect_lang() {
                $crate::i18n::Lang::En => $en,
                $crate::i18n::Lang::Zh => $zh,
            }
        }
    };
}

include!("strings_program.rs");
include!("strings_args.rs");
include!("strings_interactive.rs");
include!("strings_errors.rs");
include!("strings_logs.rs");

/// Format an error with description and detail: "error: {description}: {detail}"
pub fn fmt_error(description: &str, detail: &dyn std::fmt::Display) -> String {
    match detect_lang() {
        Lang::En => format!("error: {}: {}", description, detail),
        Lang::Zh => format!("错误：{}：{}", description, detail),
    }
}

/// Format an input-too-large error with concrete sizes.
pub fn fmt_input_too_large(size: usize, limit: usize) -> String {
    match detect_lang() {
        Lang::En => format!("error: input too large: {} bytes (limit: {})", size, limit),
        Lang::Zh => format!("错误：输入过大：{} 字节（限制：{}）", size, limit),
    }
}

/// Interactive mode: one-line success summary after writing the preview PNG.
pub fn format_compile_ok(pages: usize, bytes: usize, path: &std::path::Path) -> String {
    match detect_lang() {
        Lang::En => format!(
            "OK: {} page(s), {} bytes -> {}",
            pages,
            bytes,
            path.display()
        ),
        Lang::Zh => format!(
            "成功: {} 页，{} 字节 -> {}",
            pages,
            bytes,
            path.display()
        ),
    }
}
