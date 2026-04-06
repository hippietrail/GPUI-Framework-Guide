use gpui::{Hsla, Rgba, rgba};
use numnum_core::config::{Color, Settings};

/// Bridge between numnum_core::config colors and GPUI Hsla colors.
#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Hsla,
    pub editor_background: Hsla,
    pub _gutter: Hsla,
    pub _status_bar: Hsla,
    pub divider: Hsla,
    pub cursor: Hsla,
    pub selection: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_dimmed: Hsla,
    pub result: Hsla,
    pub error: Hsla,

    // Syntax colors
    pub syn_number: Hsla,
    pub syn_operator: Hsla,
    pub syn_keyword: Hsla,
    pub syn_function: Hsla,
    pub syn_variable: Hsla,
    pub _syn_variable_def: Hsla,
    pub syn_unit: Hsla,
    pub syn_currency: Hsla,
    pub syn_label: Hsla,
    pub syn_comment: Hsla,
    pub syn_header: Hsla,
    pub syn_percent: Hsla,
    pub _syn_string: Hsla,
    pub syn_scale: Hsla,
}

fn color_to_hsla(c: Color) -> Hsla {
    let r: Rgba = rgba(c.to_rgba_u32());
    r.into()
}

impl Theme {
    pub fn from_settings(settings: &Settings) -> Self {
        let t = &settings.theme;
        let s = &t.syntax;
        Theme {
            background: color_to_hsla(t.background),
            editor_background: color_to_hsla(t.editor_background),
            _gutter: color_to_hsla(t.gutter),
            _status_bar: color_to_hsla(t.status_bar),
            divider: color_to_hsla(t.divider),
            cursor: color_to_hsla(t.cursor),
            selection: color_to_hsla(t.selection),
            text: color_to_hsla(t.text),
            text_muted: color_to_hsla(t.text_muted),
            text_dimmed: color_to_hsla(t.text_dimmed),
            result: color_to_hsla(t.result),
            error: color_to_hsla(t.error),

            syn_number: color_to_hsla(s.number),
            syn_operator: color_to_hsla(s.operator),
            syn_keyword: color_to_hsla(s.keyword),
            syn_function: color_to_hsla(s.function),
            syn_variable: color_to_hsla(s.variable),
            _syn_variable_def: color_to_hsla(s.variable_def),
            syn_unit: color_to_hsla(s.unit),
            syn_currency: color_to_hsla(s.currency),
            syn_label: color_to_hsla(s.label),
            syn_comment: color_to_hsla(s.comment),
            syn_header: color_to_hsla(s.header),
            syn_percent: color_to_hsla(s.percent),
            _syn_string: color_to_hsla(s.string),
            syn_scale: color_to_hsla(s.scale),
        }
    }
}
