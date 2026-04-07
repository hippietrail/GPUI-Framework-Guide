use std::collections::HashMap;
use std::path::PathBuf;

/// Settings loaded from ~/.config/numnum/settings.toml (Linux/FreeBSD)
/// or ~/Library/Application Support/numnum/settings.toml (macOS)
/// or %APPDATA%/numnum/settings.toml (Windows)
#[derive(Debug, Clone)]
pub struct Settings {
    pub appearance: AppearanceSettings,
    pub editor: EditorSettings,
    pub window: WindowSettings,
}

#[derive(Debug, Clone)]
pub struct WindowSettings {
    pub width: f32,
    pub height: f32,
    pub title_bar: String, // "system", "none", "numnum"
}

#[derive(Debug, Clone)]
pub struct AppearanceSettings {
    pub mode: String,        // "dark", "light", "auto"
    pub dark_theme: String,  // theme file name without .toml
    pub light_theme: String,
}

#[derive(Debug, Clone)]
pub struct ThemeSettings {
    pub background: Color,
    pub editor_background: Color,
    pub gutter: Color,
    pub status_bar: Color,
    pub divider: Color,
    pub cursor: Color,
    pub selection: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_dimmed: Color,
    pub result: Color,
    pub error: Color,
}

#[derive(Debug, Clone)]
pub struct SyntaxColors {
    pub number: Color,
    pub operator: Color,
    pub keyword: Color,
    pub function: Color,
    pub variable: Color,
    pub variable_def: Color,
    pub unit: Color,
    pub currency: Color,
    pub label: Color,
    pub comment: Color,
    pub header: Color,
    pub percent: Color,
    pub string: Color,
    pub scale: Color,
}

#[derive(Debug, Clone)]
pub struct ThemeFile {
    pub name: String,
    pub appearance: String, // "dark" or "light"
    pub colors: ThemeSettings,
    pub syntax: SyntaxColors,
}

#[derive(Debug, Clone)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_weight: String,
    pub font_size: f32,
    pub line_height: f32,
    pub tab_size: u32,
    pub split_ratio: f32,
    pub copy_full_precision: bool,
    pub precision: u32,
    pub show_diagnostics: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let (r, g, b, a) = match hex.len() {
            6 => (
                u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
                u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
                u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
                255,
            ),
            8 => (
                u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
                u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
                u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
                u8::from_str_radix(&hex[6..8], 16).unwrap_or(0),
            ),
            _ => (0, 0, 0, 255),
        };
        Color { r, g, b, a }
    }

    pub fn to_rgba_u32(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    pub fn to_rgb_u32(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color { r: 0, g: 0, b: 0, a: 255 }
    }
}

pub fn config_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs_fallback("Library/Application Support/numnum")
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata).join("numnum")
        } else {
            dirs_fallback(".config/numnum")
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("numnum")
        } else {
            dirs_fallback(".config/numnum")
        }
    }
}

fn dirs_fallback(relative: &str) -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(relative)
    } else {
        PathBuf::from(relative)
    }
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

/// List available themes from the themes directory.
/// Returns (filename_without_extension, display_name, appearance) tuples.
pub fn list_themes() -> Vec<(String, String, String)> {
    let dir = themes_dir();
    let mut themes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                let stem = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let sections = parse_toml(&content);
                    let name = sections.get("")
                        .and_then(|s| s.get("name"))
                        .cloned()
                        .unwrap_or_else(|| stem.clone());
                    let appearance = sections.get("")
                        .and_then(|s| s.get("appearance"))
                        .cloned()
                        .unwrap_or_else(|| "dark".to_string());
                    themes.push((stem, name, appearance));
                }
            }
        }
    }
    themes.sort_by(|a, b| a.1.cmp(&b.1));
    themes
}

/// Minimal TOML parser shared by Settings and ThemeFile.
/// Returns sections mapping "section.name" -> { key -> value }.
fn parse_toml(toml_str: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::new();

    for line in toml_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len()-1].to_string();
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let mut val = line[eq_pos+1..].trim().to_string();
            // Extract quoted value: find opening and closing quotes
            if val.starts_with('"') {
                if let Some(close_quote) = val[1..].find('"') {
                    val = val[1..1 + close_quote].to_string();
                }
            } else {
                // Strip inline comments only for unquoted values
                if let Some(comment_pos) = val.find(" #") {
                    val = val[..comment_pos].trim().to_string();
                }
            }
            sections.entry(current_section.clone()).or_default().insert(key, val);
        }
    }

    sections
}

// ── ThemeFile ──────────────────────────────────────────────────────────────

impl ThemeFile {
    /// Load a theme file by name (without .toml extension) from the themes directory.
    pub fn load(name: &str) -> Self {
        let path = themes_dir().join(format!("{}.toml", name));
        if let Ok(content) = std::fs::read_to_string(&path) {
            Self::parse_theme(&content)
        } else {
            // Fallback to mocha defaults if the file can't be read
            Self::default_mocha()
        }
    }

    fn parse_theme(toml_str: &str) -> Self {
        let sections = parse_toml(toml_str);

        let get = |section: &str, key: &str, default: &str| -> String {
            sections.get(section).and_then(|s| s.get(key)).cloned().unwrap_or_else(|| default.to_string())
        };
        let get_color = |section: &str, key: &str, default: &str| -> Color {
            Color::from_hex(&get(section, key, default))
        };

        ThemeFile {
            name: get("", "name", "Catppuccin Mocha"),
            appearance: get("", "appearance", "dark"),
            colors: ThemeSettings {
                background: get_color("colors", "background", "#1e1e2e"),
                editor_background: get_color("colors", "editor_background", "#1e1e2e"),
                gutter: get_color("colors", "gutter", "#181825"),
                status_bar: get_color("colors", "status_bar", "#181825"),
                divider: get_color("colors", "divider", "#313244"),
                cursor: get_color("colors", "cursor", "#f5e0dc"),
                selection: get_color("colors", "selection", "#45475a80"),
                text: get_color("colors", "text", "#cdd6f4"),
                text_muted: get_color("colors", "text_muted", "#a6adc8"),
                text_dimmed: get_color("colors", "text_dimmed", "#6c7086"),
                result: get_color("colors", "result", "#a6e3a1"),
                error: get_color("colors", "error", "#f38ba8"),
            },
            syntax: SyntaxColors {
                number: get_color("syntax", "number", "#cdd6f4"),
                operator: get_color("syntax", "operator", "#89dceb"),
                keyword: get_color("syntax", "keyword", "#cba6f7"),
                function: get_color("syntax", "function", "#89b4fa"),
                variable: get_color("syntax", "variable", "#f9e2af"),
                variable_def: get_color("syntax", "variable_def", "#f9e2af"),
                unit: get_color("syntax", "unit", "#94e2d5"),
                currency: get_color("syntax", "currency", "#a6e3a1"),
                label: get_color("syntax", "label", "#f9e2af"),
                comment: get_color("syntax", "comment", "#6c7086"),
                header: get_color("syntax", "header", "#b4befe"),
                percent: get_color("syntax", "percent", "#f5c2e7"),
                string: get_color("syntax", "string", "#a6e3a1"),
                scale: get_color("syntax", "scale", "#fab387"),
            },
        }
    }

    pub fn default_mocha() -> Self {
        ThemeFile {
            name: "Catppuccin Mocha".to_string(),
            appearance: "dark".to_string(),
            colors: ThemeSettings {
                background: Color::from_hex("#1e1e2e"),
                editor_background: Color::from_hex("#1e1e2e"),
                gutter: Color::from_hex("#181825"),
                status_bar: Color::from_hex("#181825"),
                divider: Color::from_hex("#313244"),
                cursor: Color::from_hex("#f5e0dc"),
                selection: Color::from_hex("#45475a80"),
                text: Color::from_hex("#cdd6f4"),
                text_muted: Color::from_hex("#a6adc8"),
                text_dimmed: Color::from_hex("#6c7086"),
                result: Color::from_hex("#ABE9B3"),
                error: Color::from_hex("#F38BA8"),
            },
            syntax: SyntaxColors {
                number: Color::from_hex("#cdd6f4"),
                operator: Color::from_hex("#89DCEB"),
                keyword: Color::from_hex("#CBA6F7"),
                function: Color::from_hex("#89B4FA"),
                variable: Color::from_hex("#FAE3B0"),
                variable_def: Color::from_hex("#FAE3B0"),
                unit: Color::from_hex("#B5E8E0"),
                currency: Color::from_hex("#ABE9B3"),
                label: Color::from_hex("#FAE3B0"),
                comment: Color::from_hex("#6c7086"),
                header: Color::from_hex("#c7d1ff"),
                percent: Color::from_hex("#F5C2E7"),
                string: Color::from_hex("#ABE9B3"),
                scale: Color::from_hex("#F8BD96"),
            },
        }
    }

    pub fn default_latte() -> Self {
        ThemeFile {
            name: "Catppuccin Latte".to_string(),
            appearance: "light".to_string(),
            colors: ThemeSettings {
                background: Color::from_hex("#eff1f5"),
                editor_background: Color::from_hex("#eff1f5"),
                gutter: Color::from_hex("#e6e9ef"),
                status_bar: Color::from_hex("#e6e9ef"),
                divider: Color::from_hex("#ccd0da"),
                cursor: Color::from_hex("#dc8a78"),
                selection: Color::from_hex("#acb0be80"),
                text: Color::from_hex("#4c4f69"),
                text_muted: Color::from_hex("#6c6f85"),
                text_dimmed: Color::from_hex("#9ca0b0"),
                result: Color::from_hex("#40a02b"),
                error: Color::from_hex("#d20f39"),
            },
            syntax: SyntaxColors {
                number: Color::from_hex("#4c4f69"),
                operator: Color::from_hex("#04a5e5"),
                keyword: Color::from_hex("#8839ef"),
                function: Color::from_hex("#1e66f5"),
                variable: Color::from_hex("#df8e1d"),
                variable_def: Color::from_hex("#df8e1d"),
                unit: Color::from_hex("#179299"),
                currency: Color::from_hex("#40a02b"),
                label: Color::from_hex("#df8e1d"),
                comment: Color::from_hex("#9ca0b0"),
                header: Color::from_hex("#7287fd"),
                percent: Color::from_hex("#ea76cb"),
                string: Color::from_hex("#40a02b"),
                scale: Color::from_hex("#fe640b"),
            },
        }
    }

    fn to_toml(&self) -> String {
        let c = &self.colors;
        let s = &self.syntax;
        format!(
            r#"name = "{name}"
appearance = "{appearance}"

[colors]
background = "{background}"
editor_background = "{editor_background}"
gutter = "{gutter}"
status_bar = "{status_bar}"
divider = "{divider}"
cursor = "{cursor}"
selection = "{selection}"
text = "{text}"
text_muted = "{text_muted}"
text_dimmed = "{text_dimmed}"
result = "{result}"
error = "{error}"

[syntax]
number = "{syn_number}"
operator = "{syn_operator}"
keyword = "{syn_keyword}"
function = "{syn_function}"
variable = "{syn_variable}"
variable_def = "{syn_variable_def}"
unit = "{syn_unit}"
currency = "{syn_currency}"
label = "{syn_label}"
comment = "{syn_comment}"
header = "{syn_header}"
percent = "{syn_percent}"
string = "{syn_string}"
scale = "{syn_scale}"
"#,
            name = self.name,
            appearance = self.appearance,
            background = c.background.to_hex(),
            editor_background = c.editor_background.to_hex(),
            gutter = c.gutter.to_hex(),
            status_bar = c.status_bar.to_hex(),
            divider = c.divider.to_hex(),
            cursor = c.cursor.to_hex(),
            selection = c.selection.to_hex(),
            text = c.text.to_hex(),
            text_muted = c.text_muted.to_hex(),
            text_dimmed = c.text_dimmed.to_hex(),
            result = c.result.to_hex(),
            error = c.error.to_hex(),
            syn_number = s.number.to_hex(),
            syn_operator = s.operator.to_hex(),
            syn_keyword = s.keyword.to_hex(),
            syn_function = s.function.to_hex(),
            syn_variable = s.variable.to_hex(),
            syn_variable_def = s.variable_def.to_hex(),
            syn_unit = s.unit.to_hex(),
            syn_currency = s.currency.to_hex(),
            syn_label = s.label.to_hex(),
            syn_comment = s.comment.to_hex(),
            syn_header = s.header.to_hex(),
            syn_percent = s.percent.to_hex(),
            syn_string = s.string.to_hex(),
            syn_scale = s.scale.to_hex(),
        )
    }
}

/// Create default theme files if they don't already exist.
pub fn ensure_default_themes() {
    let dir = themes_dir();
    let _ = std::fs::create_dir_all(&dir);

    let mocha_path = dir.join("catppuccin-mocha.toml");
    if !mocha_path.exists() {
        let _ = std::fs::write(&mocha_path, ThemeFile::default_mocha().to_toml());
    }

    let latte_path = dir.join("catppuccin-latte.toml");
    if !latte_path.exists() {
        let _ = std::fs::write(&latte_path, ThemeFile::default_latte().to_toml());
    }

    let bundled: &[(&str, &str)] = &[
        ("tokyo-night.toml", include_str!("../../../themes/tokyo-night.toml")),
        ("tokyo-night-day.toml", include_str!("../../../themes/tokyo-night-day.toml")),
        ("rose-pine-dawn.toml", include_str!("../../../themes/rose-pine-dawn.toml")),
        ("rose-pine-moon.toml", include_str!("../../../themes/rose-pine-moon.toml")),
        ("zed-one-dark.toml", include_str!("../../../themes/zed-one-dark.toml")),
        ("zed-one-light.toml", include_str!("../../../themes/zed-one-light.toml")),
    ];
    for (filename, content) in bundled {
        let path = dir.join(filename);
        if !path.exists() {
            let _ = std::fs::write(&path, content);
        }
    }
}

// ── Settings ───────────────────────────────────────────────────────────────

impl Settings {
    pub fn load() -> Self {
        let path = settings_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            Self::parse(&content)
        } else {
            Self::default()
        }
    }

    fn parse(toml_str: &str) -> Self {
        let sections = parse_toml(toml_str);

        let get = |section: &str, key: &str, default: &str| -> String {
            sections.get(section).and_then(|s| s.get(key)).cloned().unwrap_or_else(|| default.to_string())
        };
        let get_f32 = |section: &str, key: &str, default: f32| -> f32 {
            sections.get(section).and_then(|s| s.get(key)).and_then(|v| v.parse().ok()).unwrap_or(default)
        };
        let get_u32 = |section: &str, key: &str, default: u32| -> u32 {
            sections.get(section).and_then(|s| s.get(key)).and_then(|v| v.parse().ok()).unwrap_or(default)
        };
        let get_bool = |section: &str, key: &str, default: bool| -> bool {
            sections.get(section).and_then(|s| s.get(key)).map(|v| v == "true").unwrap_or(default)
        };

        Settings {
            appearance: AppearanceSettings {
                mode: get("appearance", "mode", "auto"),
                dark_theme: get("appearance", "dark_theme", "catppuccin-mocha"),
                light_theme: get("appearance", "light_theme", "catppuccin-latte"),
            },
            editor: EditorSettings {
                font_family: get("editor", "font_family", "Maple Mono NF"),
                font_weight: get("editor", "font_weight", "Regular"),
                font_size: get_f32("editor", "font_size", 16.0),
                line_height: get_f32("editor", "line_height", 1.6),
                tab_size: get_u32("editor", "tab_size", 4),
                split_ratio: get_f32("editor.split", "default_ratio", 0.6),
                copy_full_precision: get_bool("editor.clipboard", "full_precision", true),
                precision: get_u32("editor", "precision", 2),
                show_diagnostics: get_bool("editor", "show_diagnostics", true),
            },
            window: WindowSettings {
                width: get_f32("window", "width", 680.0),
                height: get_f32("window", "height", 620.0),
                title_bar: get("window", "title_bar", "system"),
            },
        }
    }

    /// Write current settings back to the TOML config file.
    pub fn save(&self) {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let a = &self.appearance;
        let e = &self.editor;
        let toml = format!(
            r#"[appearance]
mode = "{mode}"
dark_theme = "{dark_theme}"
light_theme = "{light_theme}"

[editor]
font_family = "{font_family}"
font_weight = "{font_weight}"
font_size = {font_size}
line_height = {line_height}
tab_size = {tab_size}
precision = {precision}
show_diagnostics = {show_diagnostics}

[editor.split]
default_ratio = {split_ratio}

[editor.clipboard]
full_precision = {full_precision}

[window]
width = {win_width}
height = {win_height}
title_bar = "{win_title_bar}"
"#,
            mode = a.mode,
            dark_theme = a.dark_theme,
            light_theme = a.light_theme,
            font_family = e.font_family,
            font_weight = e.font_weight,
            font_size = e.font_size,
            line_height = e.line_height,
            tab_size = e.tab_size,
            precision = e.precision,
            show_diagnostics = e.show_diagnostics,
            split_ratio = e.split_ratio,
            full_precision = e.copy_full_precision,
            win_width = self.window.width,
            win_height = self.window.height,
            win_title_bar = self.window.title_bar,
        );
        let _ = std::fs::write(&path, toml);
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            appearance: AppearanceSettings {
                mode: "auto".to_string(),
                dark_theme: "catppuccin-mocha".to_string(),
                light_theme: "catppuccin-latte".to_string(),
            },
            editor: EditorSettings {
                font_family: "Maple Mono NF".to_string(),
                font_weight: "Regular".to_string(),
                font_size: 16.0,
                line_height: 1.6,
                tab_size: 4,
                split_ratio: 0.6,
                copy_full_precision: true,
                precision: 2,
                show_diagnostics: true,
            },
            window: WindowSettings {
                width: 680.0,
                height: 620.0,
                title_bar: "system".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let c = Color::from_hex("#1e1e2e");
        assert_eq!(c.r, 0x1e);
        assert_eq!(c.g, 0x1e);
        assert_eq!(c.b, 0x2e);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_color_from_hex_with_alpha() {
        let c = Color::from_hex("#45475a80");
        assert_eq!(c.r, 0x45);
        assert_eq!(c.g, 0x47);
        assert_eq!(c.b, 0x5a);
        assert_eq!(c.a, 0x80);
    }

    #[test]
    fn test_config_dir() {
        let dir = config_dir();
        assert!(dir.to_string_lossy().contains("numnum"));
    }

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert_eq!(s.appearance.mode, "auto");
        assert_eq!(s.appearance.dark_theme, "catppuccin-mocha");
        assert_eq!(s.appearance.light_theme, "catppuccin-latte");
        assert_eq!(s.editor.font_family, "Maple Mono NF");
        assert_eq!(s.editor.font_size, 16.0);
        assert_eq!(s.editor.split_ratio, 0.6);
        assert_eq!(s.editor.show_diagnostics, true);
    }

    #[test]
    fn test_parse_settings_new_format() {
        let toml = r##"
[appearance]
mode = "dark"
dark_theme = "catppuccin-mocha"
light_theme = "catppuccin-latte"

[editor]
font_family = "Fira Code"
font_size = 14
show_diagnostics = false
"##;
        let s = Settings::parse(toml);
        assert_eq!(s.appearance.mode, "dark");
        assert_eq!(s.appearance.dark_theme, "catppuccin-mocha");
        assert_eq!(s.editor.font_family, "Fira Code");
        assert_eq!(s.editor.font_size, 14.0);
        assert_eq!(s.editor.show_diagnostics, false);
        // Non-specified values should use defaults
        assert_eq!(s.editor.split_ratio, 0.6);
    }

    #[test]
    fn test_parse_settings_legacy_format() {
        // Old settings.toml with [theme] section — should still parse editor settings
        // and default appearance to auto/mocha/latte
        let toml = r##"
[theme]
name = "catppuccin-mocha"
background = "#1e1e2e"

[theme.syntax]
number = "#cdd6f4"

[editor]
font_family = "Fira Code"
font_size = 14
"##;
        let s = Settings::parse(toml);
        // [theme] section is just ignored — appearance gets defaults
        assert_eq!(s.appearance.mode, "auto");
        assert_eq!(s.appearance.dark_theme, "catppuccin-mocha");
        assert_eq!(s.editor.font_family, "Fira Code");
        assert_eq!(s.editor.font_size, 14.0);
    }

    #[test]
    fn test_theme_file_default_mocha() {
        let tf = ThemeFile::default_mocha();
        assert_eq!(tf.name, "Catppuccin Mocha");
        assert_eq!(tf.appearance, "dark");
        assert_eq!(tf.colors.background.r, 0x1e);
    }

    #[test]
    fn test_theme_file_default_latte() {
        let tf = ThemeFile::default_latte();
        assert_eq!(tf.name, "Catppuccin Latte");
        assert_eq!(tf.appearance, "light");
        assert_eq!(tf.colors.background.r, 0xef);
    }

    #[test]
    fn test_theme_file_parse() {
        let toml = r##"
name = "Test Theme"
appearance = "dark"

[colors]
background = "#ff0000"
text = "#00ff00"

[syntax]
number = "#0000ff"
"##;
        let tf = ThemeFile::parse_theme(toml);
        assert_eq!(tf.name, "Test Theme");
        assert_eq!(tf.appearance, "dark");
        assert_eq!(tf.colors.background.r, 0xff);
        assert_eq!(tf.colors.background.g, 0x00);
        assert_eq!(tf.colors.text.r, 0x00);
        assert_eq!(tf.colors.text.g, 0xff);
        assert_eq!(tf.syntax.number.b, 0xff);
    }

    #[test]
    fn test_load_real_settings() {
        let _s = Settings::load();
        // Just ensure it doesn't panic, regardless of file format on disk
    }
}
