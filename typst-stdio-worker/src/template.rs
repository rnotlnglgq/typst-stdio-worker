/// Template wrapping for typst source code.

use std::fmt;
use std::io;
use std::path::Path;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum TemplateKind {
    /// No wrapping — compile source as-is.
    #[default]
    Raw,
    /// Template via quiconf package.
    Quiconf,
    /// Default Chinese-optimized template (legacy structured settings).
    #[allow(dead_code)]
    DeprecatedDefault,
}

static QUICONF_PRELUDE: OnceLock<String> = OnceLock::new();
static DEPRECATED_DEFAULT_CONFIG: OnceLock<TemplateConfig> = OnceLock::new();

fn quiconf_prelude_cached() -> &'static str {
    QUICONF_PRELUDE
        .get_or_init(|| {
            crate::prelude_loader::load_utf8(
                "prelude/quiconf.typ",
                include_str!("../resources/prelude/quiconf.typ"),
            )
        })
        .as_str()
}

fn deprecated_default_cached() -> &'static TemplateConfig {
    DEPRECATED_DEFAULT_CONFIG.get_or_init(|| {
        let raw = crate::prelude_loader::load_utf8(
            "prelude/deprecated_default.typ",
            include_str!("../resources/prelude/deprecated_default.typ"),
        );
        TemplateConfig::from_typ_str(&raw)
    })
}

impl TemplateKind {
    /// Apply this template to user source code.
    ///
    /// Returns `(wrapped_source, prelude_line_count)`. The prelude line count is
    /// the number of lines injected before the user's content, used by the renderer
    /// to shift diagnostic line numbers back into the user's coordinate system.
    ///
    /// `Quiconf` and `DeprecatedDefault` preludes are read from the local resource tree
    /// (see [`crate::prelude_loader::resource_root`]), with compile-time fallbacks when
    /// files are absent.
    pub fn apply_to(self, source: &str) -> (String, usize) {
        match self {
            Self::Raw => (source.to_string(), 0),
            Self::DeprecatedDefault => deprecated_default_cached().apply_to(source),
            Self::Quiconf => wrap_with_prelude(quiconf_prelude_cached(), source),
        }
    }
}

fn wrap_with_prelude(prelude: &str, content: &str) -> (String, usize) {
    let prelude_lines = prelude.matches('\n').count();
    let mut wrapped = String::with_capacity(prelude.len() + content.len());
    wrapped.push_str(prelude);
    wrapped.push_str(content);
    (wrapped, prelude_lines)
}
// ---------------------------------------------------------------------------
// Types & methods below are not currently used,
// but are prepared for future protocol/API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageImport {
    pub namespace: String,
    pub name: String,
    pub version: String,
}

impl PackageImport {
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: version.into(),
        }
    }

    /// Generate a typst `#import` line (without trailing newline).
    pub fn to_typst(&self) -> String {
        format!(
            "#import \"@{}/{}:{}\"",
            self.namespace, self.name, self.version
        )
    }

    /// Parse a typst `#import "@namespace/name:version"` line.
    ///
    /// Accepts optional leading/trailing whitespace and trailing content after
    /// the closing quote (e.g. `: *` for glob imports).
    #[allow(dead_code)]
    pub fn parse(line: &str) -> Option<Self> {
        let rest = line.trim().strip_prefix("#import")?.trim_start();
        let rest = rest.strip_prefix('"')?.strip_prefix('@')?;
        let spec = &rest[..rest.find('"')?];

        let slash = spec.find('/')?;
        let namespace = &spec[..slash];
        let after_slash = &spec[slash + 1..];
        let colon = after_slash.find(':')?;
        let name = &after_slash[..colon];
        let version = &after_slash[colon + 1..];

        if namespace.is_empty() || name.is_empty() || version.is_empty() {
            return None;
        }

        Some(Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        })
    }
}

impl fmt::Display for PackageImport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}/{}:{}", self.namespace, self.name, self.version)
    }
}

// ---------------------------------------------------------------------------
// Structured settings (programmatic prelude building)
// ---------------------------------------------------------------------------

/// Generate a `#set directive(key: val, ...)` rule, choosing single-line or
/// multi-line format based on estimated length.
#[allow(dead_code)]
fn format_set_rule(directive: &str, parts: &[String]) -> String {
    if parts.is_empty() {
        return String::new();
    }
    let single = format!("#set {}({})\n", directive, parts.join(", "));
    if single.len() <= 72 || parts.len() == 1 {
        single
    } else {
        format!("#set {}(\n  {}\n)\n", directive, parts.join(",\n  "))
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
}

impl Default for PageSettings {
    fn default() -> Self {
        Self {
            width: Some("auto".into()),
            height: Some("auto".into()),
            margin: Some("10pt".into()),
        }
    }
}

impl PageSettings {
    pub fn to_typst(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref w) = self.width {
            parts.push(format!("width: {w}"));
        }
        if let Some(ref h) = self.height {
            parts.push(format!("height: {h}"));
        }
        if let Some(ref m) = self.margin {
            parts.push(format!("margin: {m}"));
        }
        format_set_rule("page", &parts)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSettings {
    #[serde(default)]
    pub font: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

impl Default for TextSettings {
    fn default() -> Self {
        Self {
            font: vec![
                "Latin Modern Roman".into(),
                "Noto Serif CJK SC".into(),
                "Source Han Serif SC".into(),
                "SimSun".into(),
            ],
            size: Some("10.5pt".into()),
            lang: Some("zh".into()),
            region: Some("cn".into()),
        }
    }
}

impl TextSettings {
    pub fn to_typst(&self) -> String {
        let mut parts = Vec::new();
        if !self.font.is_empty() {
            if self.font.len() == 1 {
                parts.push(format!("font: \"{}\"", self.font[0]));
            } else {
                let list: Vec<_> = self.font.iter().map(|f| format!("\"{f}\"")).collect();
                parts.push(format!("font: ({})", list.join(", ")));
            }
        }
        if let Some(ref s) = self.size {
            parts.push(format!("size: {s}"));
        }
        if let Some(ref l) = self.lang {
            parts.push(format!("lang: \"{l}\""));
        }
        if let Some(ref r) = self.region {
            parts.push(format!("region: \"{r}\""));
        }
        format_set_rule("text", &parts)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub math_font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_above: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_below: Option<String>,
}

impl Default for MathSettings {
    fn default() -> Self {
        Self {
            math_font: Some("Latin Modern Math".into()),
            block_above: Some("1em".into()),
            block_below: Some("1em".into()),
        }
    }
}

impl MathSettings {
    pub fn to_typst(&self) -> String {
        let mut out = String::new();
        if let Some(ref f) = self.math_font {
            out.push_str(&format!(
                "#show math.equation: set text(font: \"{f}\")\n"
            ));
        }
        let block_parts: Vec<String> = [
            self.block_above
                .as_ref()
                .map(|v| format!("above: {v}")),
            self.block_below
                .as_ref()
                .map(|v| format!("below: {v}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !block_parts.is_empty() {
            out.push_str(&format!(
                "#show math.equation.where(block: true): set block({})\n",
                block_parts.join(", ")
            ));
        }
        out
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_line_indent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leading: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<String>,
}

impl Default for ParagraphSettings {
    fn default() -> Self {
        Self {
            justify: Some(true),
            first_line_indent: Some("2em".into()),
            leading: Some("0.65em".into()),
            spacing: Some("0.65em".into()),
        }
    }
}

impl ParagraphSettings {
    pub fn to_typst(&self) -> String {
        let mut parts = Vec::new();
        if let Some(j) = self.justify {
            parts.push(format!("justify: {j}"));
        }
        if let Some(ref i) = self.first_line_indent {
            parts.push(format!("first-line-indent: {i}"));
        }
        if let Some(ref l) = self.leading {
            parts.push(format!("leading: {l}"));
        }
        if let Some(ref s) = self.spacing {
            parts.push(format!("spacing: {s}"));
        }
        format_set_rule("par", &parts)
    }
}

/// Structured document settings that can be built programmatically.
///
/// Each sub-field covers a category of typst `#set` / `#show` rules. The
/// `extra_rules` vector provides an escape hatch for arbitrary rules that
/// don't fit the structured fields.
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredSettings {
    #[serde(default)]
    pub page: PageSettings,
    #[serde(default)]
    pub text: TextSettings,
    #[serde(default)]
    pub math: MathSettings,
    #[serde(default)]
    pub paragraph: ParagraphSettings,
    #[serde(default)]
    pub extra_rules: Vec<String>,
}

impl StructuredSettings {
    pub fn to_typst(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.page.to_typst());
        out.push_str(&self.text.to_typst());
        out.push_str(&self.math.to_typst());
        out.push_str(&self.paragraph.to_typst());
        for rule in &self.extra_rules {
            out.push_str(rule);
            if !rule.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }

    // -- Builder methods (not called yet; prepared for future protocol/API) --

    #[allow(dead_code)]
    pub fn set_body_fonts(&mut self, fonts: Vec<String>) -> &mut Self {
        self.text.font = fonts;
        self
    }

    #[allow(dead_code)]
    pub fn set_math_font(&mut self, font: impl Into<String>) -> &mut Self {
        self.math.math_font = Some(font.into());
        self
    }

    #[allow(dead_code)]
    pub fn set_text_size(&mut self, size: impl Into<String>) -> &mut Self {
        self.text.size = Some(size.into());
        self
    }

    #[allow(dead_code)]
    pub fn set_language(
        &mut self,
        lang: impl Into<String>,
        region: Option<String>,
    ) -> &mut Self {
        self.text.lang = Some(lang.into());
        self.text.region = region;
        self
    }

    #[allow(dead_code)]
    pub fn set_page_auto(&mut self, margin: impl Into<String>) -> &mut Self {
        self.page.width = Some("auto".into());
        self.page.height = Some("auto".into());
        self.page.margin = Some(margin.into());
        self
    }

    #[allow(dead_code)]
    pub fn set_paragraph(
        &mut self,
        justify: bool,
        indent: Option<String>,
        leading: Option<String>,
        spacing: Option<String>,
    ) -> &mut Self {
        self.paragraph.justify = Some(justify);
        self.paragraph.first_line_indent = indent;
        self.paragraph.leading = leading;
        self.paragraph.spacing = spacing;
        self
    }
}

/// Settings payload: either a raw typst snippet or a structured representation.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SettingsBlock {
    /// Raw typst code loaded from a `.typ` file.
    #[allow(dead_code)]
    Raw(String),
    /// Programmatically constructed settings.
    Structured(StructuredSettings),
}

impl Default for SettingsBlock {
    fn default() -> Self {
        Self::Structured(StructuredSettings::default())
    }
}

impl SettingsBlock {
    pub fn to_typst(&self) -> String {
        match self {
            Self::Raw(s) => {
                if s.is_empty() || s.ends_with('\n') {
                    s.clone()
                } else {
                    format!("{s}\n")
                }
            }
            Self::Structured(s) => s.to_typst(),
        }
    }
}

/// A complete template configuration: package imports + document settings.
///
/// The user's content is *not* part of this struct; it is appended by
/// [`TemplateConfig::apply_to`].
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TemplateConfig {
    pub imports: Vec<PackageImport>,
    pub settings: SettingsBlock,
}

impl Default for TemplateConfig {
    /// The built-in Chinese-optimized default (physica package, Noto Serif CJK
    /// SC fonts, 10.5 pt body text, justified paragraphs with 2 em indent).
    fn default() -> Self {
        Self {
            imports: vec![PackageImport::new("preview", "physica", "0.9.3")],
            settings: SettingsBlock::default(),
        }
    }
}

impl TemplateConfig {
    /// Load a template configuration from a `.typ` file.
    ///
    /// Lines matching `#import "@..."` are extracted into structured
    /// [`PackageImport`] entries; everything else becomes a
    /// [`SettingsBlock::Raw`].
    #[allow(dead_code)]
    pub fn from_typ_file(path: &Path) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_typ_str(&content))
    }

    /// Parse template configuration from a typst source string.
    ///
    /// Same splitting logic as [`from_typ_file`](Self::from_typ_file).
    #[allow(dead_code)]
    pub fn from_typ_str(content: &str) -> Self {
        let mut imports = Vec::new();
        let mut settings_lines = Vec::new();

        for line in content.lines() {
            if let Some(pkg) = PackageImport::parse(line) {
                imports.push(pkg);
            } else {
                settings_lines.push(line);
            }
        }

        while settings_lines.last().is_some_and(|l| l.trim().is_empty()) {
            settings_lines.pop();
        }

        let raw = if settings_lines.is_empty() {
            String::new()
        } else {
            let mut s = settings_lines.join("\n");
            s.push('\n');
            s
        };

        Self {
            imports,
            settings: SettingsBlock::Raw(raw),
        }
    }

    /// Generate the complete typst prelude string (imports + settings).
    ///
    /// The returned string always ends with `\n` so that user content starts
    /// on a fresh line.
    pub fn to_typst_prelude(&self) -> String {
        let mut out = String::new();
        for imp in &self.imports {
            out.push_str(&imp.to_typst());
            out.push('\n');
        }
        out.push_str(&self.settings.to_typst());
        out
    }

    /// Apply this configuration to user source code (prelude + user content).
    ///
    /// Same return semantics as [`TemplateKind::apply_to`].
    #[allow(dead_code)]
    pub fn apply_to(&self, source: &str) -> (String, usize) {
        wrap_with_prelude(&self.to_typst_prelude(), source)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_has_zero_prelude() {
        let (s, n) = TemplateKind::Raw.apply_to("Hello");
        assert_eq!(s, "Hello");
        assert_eq!(n, 0);
    }

    #[test]
    fn default_prelude_lines_match_actual() {
        let (s, n) = TemplateKind::DeprecatedDefault.apply_to("USER");
        assert!(s.ends_with("USER"));
        let user_offset = s.find("USER").unwrap();
        let line_of_user = s[..user_offset].matches('\n').count();
        assert_eq!(
            line_of_user, n,
            "user content line must be exactly prelude line count"
        );
    }

    // -- PackageImport --

    #[test]
    fn package_import_to_typst() {
        let imp = PackageImport::new("preview", "physica", "0.9.3");
        assert_eq!(imp.to_typst(), r#"#import "@preview/physica:0.9.3""#);
    }

    #[test]
    fn package_import_parse_roundtrip() {
        let imp = PackageImport::new("preview", "physica", "0.9.3");
        let line = imp.to_typst();
        let parsed = PackageImport::parse(&line).expect("should parse");
        assert_eq!(imp, parsed);
    }

    #[test]
    fn package_import_parse_with_glob() {
        let line = r#"#import "@preview/cetz:0.3.4": *"#;
        let parsed = PackageImport::parse(line).expect("should parse");
        assert_eq!(parsed.namespace, "preview");
        assert_eq!(parsed.name, "cetz");
        assert_eq!(parsed.version, "0.3.4");
    }

    #[test]
    fn package_import_parse_rejects_non_package() {
        assert!(PackageImport::parse("#set text(size: 10pt)").is_none());
        assert!(PackageImport::parse("").is_none());
        assert!(PackageImport::parse(r#"#import "local-file.typ""#).is_none());
    }

    #[test]
    fn package_import_serde_roundtrip() {
        let imp = PackageImport::new("preview", "physica", "0.9.3");
        let json = serde_json::to_string(&imp).unwrap();
        let back: PackageImport = serde_json::from_str(&json).unwrap();
        assert_eq!(imp, back);
    }

    #[test]
    fn package_import_display() {
        let imp = PackageImport::new("preview", "physica", "0.9.3");
        assert_eq!(format!("{imp}"), "@preview/physica:0.9.3");
    }

    // -- StructuredSettings --

    #[test]
    fn default_structured_settings_generates_expected_rules() {
        let typst = StructuredSettings::default().to_typst();
        assert!(typst.contains("#set page("));
        assert!(typst.contains("width: auto"));
        assert!(typst.contains("#set text("));
        assert!(typst.contains("\"Latin Modern Roman\""));
        assert!(typst.contains("\"Noto Serif CJK SC\""));
        assert!(typst.contains("size: 10.5pt"));
        assert!(typst.contains("#show math.equation: set text(font: \"Latin Modern Math\")"));
        assert!(typst.contains(
            "#show math.equation.where(block: true): set block(above: 1em, below: 1em)"
        ));
        assert!(typst.contains("#set par("));
        assert!(typst.contains("justify: true"));
        assert!(typst.contains("first-line-indent: 2em"));
    }

    #[test]
    fn structured_settings_builder_modifies_output() {
        let mut s = StructuredSettings::default();
        s.set_body_fonts(vec!["Arial".into()])
            .set_math_font("XITS Math")
            .set_text_size("12pt")
            .set_language("en", None);

        let typst = s.to_typst();
        assert!(typst.contains("font: \"Arial\""));
        assert!(typst.contains("font: \"XITS Math\""));
        assert!(typst.contains("size: 12pt"));
        assert!(typst.contains("lang: \"en\""));
        assert!(!typst.contains("region:"));
    }

    #[test]
    fn extra_rules_are_appended() {
        let mut s = StructuredSettings::default();
        s.extra_rules
            .push("#set heading(numbering: \"1.\")".into());
        let typst = s.to_typst();
        assert!(typst.contains("#set heading(numbering: \"1.\")"));
    }

    // -- Sub-settings edge cases --

    #[test]
    fn empty_page_settings_produces_nothing() {
        let page = PageSettings {
            width: None,
            height: None,
            margin: None,
        };
        assert_eq!(page.to_typst(), "");
    }

    #[test]
    fn single_font_uses_bare_string() {
        let text = TextSettings {
            font: vec!["Arial".into()],
            size: None,
            lang: None,
            region: None,
        };
        let typst = text.to_typst();
        assert!(typst.contains("font: \"Arial\""));
        assert!(
            !typst.contains("font: ("),
            "single font should not use tuple syntax"
        );
    }

    #[test]
    fn math_settings_partial() {
        let math = MathSettings {
            math_font: None,
            block_above: Some("0.5em".into()),
            block_below: None,
        };
        let typst = math.to_typst();
        assert!(!typst.contains("set text(font:"));
        assert!(typst.contains("above: 0.5em"));
        assert!(!typst.contains("below:"));
    }

    // -- TemplateConfig --

    #[test]
    fn default_config_prelude_contains_all_sections() {
        let prelude = TemplateConfig::default().to_typst_prelude();
        assert!(prelude.contains("#import \"@preview/physica:0.9.3\""));
        assert!(prelude.contains("#set page("));
        assert!(prelude.contains("#set text("));
        assert!(prelude.contains("#show math.equation"));
        assert!(prelude.contains("#set par("));
    }

    #[test]
    fn from_typ_str_separates_imports_from_settings() {
        let input = "\
#import \"@preview/physica:0.9.3\"
#import \"@preview/cetz:0.3.4\": *
#set page(width: auto, height: auto, margin: 10pt)
#set text(size: 10.5pt)
";
        let config = TemplateConfig::from_typ_str(input);
        assert_eq!(config.imports.len(), 2);
        assert_eq!(config.imports[0].name, "physica");
        assert_eq!(config.imports[1].name, "cetz");
        match &config.settings {
            SettingsBlock::Raw(s) => {
                assert!(s.contains("#set page("));
                assert!(s.contains("#set text("));
                assert!(!s.contains("#import"));
            }
            SettingsBlock::Structured(_) => panic!("expected Raw settings from from_typ_str"),
        }
    }

    #[test]
    fn from_typ_str_no_imports() {
        let input = "#set text(size: 12pt)\n";
        let config = TemplateConfig::from_typ_str(input);
        assert!(config.imports.is_empty());
        match &config.settings {
            SettingsBlock::Raw(s) => assert!(s.contains("#set text(")),
            SettingsBlock::Structured(_) => panic!("expected Raw"),
        }
    }

    #[test]
    fn from_typ_str_roundtrip_through_prelude() {
        let config = TemplateConfig::from_typ_str(
            "#import \"@preview/physica:0.9.3\"\n#set text(size: 10pt)\n",
        );
        let prelude = config.to_typst_prelude();
        assert!(prelude.starts_with("#import \"@preview/physica:0.9.3\"\n"));
        assert!(prelude.contains("#set text(size: 10pt)"));
    }

    #[test]
    fn apply_template_with_config_works() {
        let mut config = TemplateConfig::default();
        config
            .imports
            .push(PackageImport::new("preview", "cetz", "0.3.4"));
        let (wrapped, n) = config.apply_to("content");
        assert!(wrapped.contains("#import \"@preview/cetz:0.3.4\""));
        assert!(wrapped.ends_with("content"));
        assert!(n > 0);
    }

    #[test]
    fn apply_with_config_prelude_lines_correct() {
        let config = TemplateConfig::default();
        let (s, n) = config.apply_to("USER");
        assert!(s.ends_with("USER"));
        let user_offset = s.find("USER").unwrap();
        let line_of_user = s[..user_offset].matches('\n').count();
        assert_eq!(line_of_user, n);
    }
}
