use std::collections::HashMap;
use std::path::PathBuf;

/// Settings loaded from ~/.config/numnum/settings.toml (Linux/FreeBSD)
/// or ~/Library/Application Support/numnum/settings.toml (macOS)
/// or %APPDATA%/numnum/settings.toml (Windows)
#[derive(Debug, Clone)]
pub struct Settings {
    pub theme: ThemeSettings,
    pub editor: EditorSettings,
}

#[derive(Debug, Clone)]
pub struct ThemeSettings {
    pub name: String,
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
    pub syntax: SyntaxColors,
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
pub struct EditorSettings {
    pub font_family: String,
    pub font_weight: String,
    pub font_size: f32,
    pub line_height: f32,
    pub tab_size: u32,
    pub split_ratio: f32,
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
        // Minimal TOML parser for our specific format.
        // We parse key = "value" pairs grouped by [section] headers.
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
                // Strip quotes first
                let was_quoted = val.starts_with('"') && val.ends_with('"') && val.len() >= 2;
                if was_quoted {
                    val = val[1..val.len()-1].to_string();
                } else {
                    // Strip inline comments only for unquoted values
                    if let Some(comment_pos) = val.find(" #") {
                        val = val[..comment_pos].trim().to_string();
                    }
                }
                sections.entry(current_section.clone()).or_default().insert(key, val);
            }
        }

        let get = |section: &str, key: &str, default: &str| -> String {
            sections.get(section).and_then(|s| s.get(key)).cloned().unwrap_or_else(|| default.to_string())
        };
        let get_color = |section: &str, key: &str, default: &str| -> Color {
            Color::from_hex(&get(section, key, default))
        };
        let get_f32 = |section: &str, key: &str, default: f32| -> f32 {
            sections.get(section).and_then(|s| s.get(key)).and_then(|v| v.parse().ok()).unwrap_or(default)
        };
        let get_u32 = |section: &str, key: &str, default: u32| -> u32 {
            sections.get(section).and_then(|s| s.get(key)).and_then(|v| v.parse().ok()).unwrap_or(default)
        };

        Settings {
            theme: ThemeSettings {
                name: get("theme", "name", "catppuccin-mocha"),
                background: get_color("theme", "background", "#1e1e2e"),
                editor_background: get_color("theme", "editor_background", "#1e1e2e"),
                gutter: get_color("theme", "gutter", "#181825"),
                status_bar: get_color("theme", "status_bar", "#181825"),
                divider: get_color("theme", "divider", "#313244"),
                cursor: get_color("theme", "cursor", "#f5e0dc"),
                selection: get_color("theme", "selection", "#45475a80"),
                text: get_color("theme", "text", "#cdd6f4"),
                text_muted: get_color("theme", "text_muted", "#a6adc8"),
                text_dimmed: get_color("theme", "text_dimmed", "#6c7086"),
                result: get_color("theme", "result", "#a6e3a1"),
                error: get_color("theme", "error", "#f38ba8"),
                syntax: SyntaxColors {
                    number: get_color("theme.syntax", "number", "#fab387"),
                    operator: get_color("theme.syntax", "operator", "#89dceb"),
                    keyword: get_color("theme.syntax", "keyword", "#cba6f7"),
                    function: get_color("theme.syntax", "function", "#89b4fa"),
                    variable: get_color("theme.syntax", "variable", "#cdd6f4"),
                    variable_def: get_color("theme.syntax", "variable_def", "#f9e2af"),
                    unit: get_color("theme.syntax", "unit", "#94e2d5"),
                    currency: get_color("theme.syntax", "currency", "#a6e3a1"),
                    label: get_color("theme.syntax", "label", "#f9e2af"),
                    comment: get_color("theme.syntax", "comment", "#6c7086"),
                    header: get_color("theme.syntax", "header", "#b4befe"),
                    percent: get_color("theme.syntax", "percent", "#f5c2e7"),
                    string: get_color("theme.syntax", "string", "#a6e3a1"),
                    scale: get_color("theme.syntax", "scale", "#fab387"),
                },
            },
            editor: EditorSettings {
                font_family: get("editor", "font_family", "Maple Mono NF"),
                font_weight: get("editor", "font_weight", "Regular"),
                font_size: get_f32("editor", "font_size", 16.0),
                line_height: get_f32("editor", "line_height", 1.6),
                tab_size: get_u32("editor", "tab_size", 4),
                split_ratio: get_f32("editor.split", "default_ratio", 0.6),
            },
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        // Catppuccin Mocha defaults
        Settings {
            theme: ThemeSettings {
                name: "catppuccin-mocha".to_string(),
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
                result: Color::from_hex("#a6e3a1"),
                error: Color::from_hex("#f38ba8"),
                syntax: SyntaxColors {
                    number: Color::from_hex("#fab387"),
                    operator: Color::from_hex("#89dceb"),
                    keyword: Color::from_hex("#cba6f7"),
                    function: Color::from_hex("#89b4fa"),
                    variable: Color::from_hex("#cdd6f4"),
                    variable_def: Color::from_hex("#f9e2af"),
                    unit: Color::from_hex("#94e2d5"),
                    currency: Color::from_hex("#a6e3a1"),
                    label: Color::from_hex("#f9e2af"),
                    comment: Color::from_hex("#6c7086"),
                    header: Color::from_hex("#b4befe"),
                    percent: Color::from_hex("#f5c2e7"),
                    string: Color::from_hex("#a6e3a1"),
                    scale: Color::from_hex("#fab387"),
                },
            },
            editor: EditorSettings {
                font_family: "Maple Mono NF".to_string(),
                font_weight: "Regular".to_string(),
                font_size: 16.0,
                line_height: 1.6,
                tab_size: 4,
                split_ratio: 0.6,
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
        assert_eq!(s.theme.name, "catppuccin-mocha");
        assert_eq!(s.editor.font_family, "Maple Mono NF");
        assert_eq!(s.editor.font_size, 16.0);
        assert_eq!(s.editor.split_ratio, 0.6);
    }

    #[test]
    fn test_parse_settings() {
        let toml = r##"
[theme]
name = "custom"
background = "#ff0000"

[editor]
font_family = "Fira Code"
font_size = 14
"##;
        let s = Settings::parse(toml);
        assert_eq!(s.theme.name, "custom");
        assert_eq!(s.theme.background.r, 0xff);
        assert_eq!(s.theme.background.g, 0x00);
        assert_eq!(s.editor.font_family, "Fira Code");
        assert_eq!(s.editor.font_size, 14.0);
        // Non-specified values should use defaults
        assert_eq!(s.editor.split_ratio, 0.6);
    }

    #[test]
    fn test_load_real_settings() {
        // This test loads from the actual config file if it exists
        let s = Settings::load();
        // Should at least not panic
        assert!(!s.theme.name.is_empty());
    }
}
