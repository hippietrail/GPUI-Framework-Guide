use gpui::{Context, MouseButton, MouseUpEvent, Render, Window, div, prelude::*, px};
use numnum_core::config::Settings;

use crate::theme::Theme;

pub struct SettingsPane {
    pub visible: bool,
    settings: Settings,
    theme: Theme,
}

impl SettingsPane {
    pub fn new(settings: Settings, theme: Theme) -> Self {
        SettingsPane {
            visible: false,
            settings,
            theme,
        }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        cx.notify();
    }

    pub fn current_settings(&self) -> Settings {
        self.settings.clone()
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        self.settings.save();
        cx.notify();
    }

    fn inc_font_size(&mut self, cx: &mut Context<Self>) {
        self.settings.editor.font_size = (self.settings.editor.font_size + 1.0).min(72.0);
        self.settings.save();
        cx.notify();
    }

    fn dec_font_size(&mut self, cx: &mut Context<Self>) {
        self.settings.editor.font_size = (self.settings.editor.font_size - 1.0).max(8.0);
        self.settings.save();
        cx.notify();
    }

    fn inc_precision(&mut self, cx: &mut Context<Self>) {
        self.settings.editor.precision = (self.settings.editor.precision + 1).min(10);
        self.settings.save();
        cx.notify();
    }

    fn dec_precision(&mut self, cx: &mut Context<Self>) {
        self.settings.editor.precision = self.settings.editor.precision.saturating_sub(1);
        self.settings.save();
        cx.notify();
    }

    fn inc_tab_size(&mut self, cx: &mut Context<Self>) {
        self.settings.editor.tab_size = (self.settings.editor.tab_size + 1).min(8);
        self.settings.save();
        cx.notify();
    }

    fn dec_tab_size(&mut self, cx: &mut Context<Self>) {
        self.settings.editor.tab_size = self.settings.editor.tab_size.max(2) - 1;
        self.settings.save();
        cx.notify();
    }

    fn toggle_copy_full_precision(&mut self, cx: &mut Context<Self>) {
        self.settings.editor.copy_full_precision = !self.settings.editor.copy_full_precision;
        self.settings.save();
        cx.notify();
    }
}

impl Render for SettingsPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        let theme = self.theme.clone();
        let settings = &self.settings;

        // Build setting rows
        let font_family_val = settings.editor.font_family.clone();
        let font_size_val = format!("{}", settings.editor.font_size);
        let precision_val = format!("{}", settings.editor.precision);
        let line_height_val = format!("{}", settings.editor.line_height);
        let tab_size_val = format!("{}", settings.editor.tab_size);
        let copy_fp_val = if settings.editor.copy_full_precision { "Yes" } else { "No" };

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .justify_center()
            .items_center()
            .bg(gpui::hsla(0.0, 0.0, 0.0, 0.5))
            .child(
                div()
                    .w(px(400.))
                    .bg(theme.editor_background)
                    .rounded(px(12.))
                    .p(px(24.))
                    .border_1()
                    .border_color(theme.divider)
                    .flex()
                    .flex_col()
                    .gap_1()
                    // Title row with close button
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .pb(px(12.))
                            .child(
                                div()
                                    .text_color(theme.text)
                                    .text_size(px(16.))
                                    .child("Settings"),
                            )
                            .child(
                                div()
                                    .id("settings-close")
                                    .cursor_pointer()
                                    .text_color(theme.text_dimmed)
                                    .text_size(px(18.))
                                    .hover(|s| s.text_color(theme.text_muted))
                                    .child("\u{00d7}")
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                            this.close(cx);
                                        }),
                                    ),
                            ),
                    )
                    // Divider
                    .child(
                        div().w_full().h(px(1.)).bg(theme.divider).mb(px(8.)),
                    )
                    // Font Family (display only)
                    .child(
                        setting_row(
                            &theme,
                            "Font Family",
                            div().text_color(theme.text).child(font_family_val),
                        ),
                    )
                    // Font Size (+/-)
                    .child(
                        setting_row(
                            &theme,
                            "Font Size",
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .id("fs-dec")
                                        .cursor_pointer()
                                        .text_color(theme.text_muted)
                                        .hover(|s| s.text_color(theme.text))
                                        .child("-")
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                                this.dec_font_size(cx);
                                            }),
                                        ),
                                )
                                .child(div().text_color(theme.text).child(font_size_val))
                                .child(
                                    div()
                                        .id("fs-inc")
                                        .cursor_pointer()
                                        .text_color(theme.text_muted)
                                        .hover(|s| s.text_color(theme.text))
                                        .child("+")
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                                this.inc_font_size(cx);
                                            }),
                                        ),
                                ),
                        ),
                    )
                    // Precision (+/-)
                    .child(
                        setting_row(
                            &theme,
                            "Precision",
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .id("prec-dec")
                                        .cursor_pointer()
                                        .text_color(theme.text_muted)
                                        .hover(|s| s.text_color(theme.text))
                                        .child("-")
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                                this.dec_precision(cx);
                                            }),
                                        ),
                                )
                                .child(div().text_color(theme.text).child(precision_val))
                                .child(
                                    div()
                                        .id("prec-inc")
                                        .cursor_pointer()
                                        .text_color(theme.text_muted)
                                        .hover(|s| s.text_color(theme.text))
                                        .child("+")
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                                this.inc_precision(cx);
                                            }),
                                        ),
                                ),
                        ),
                    )
                    // Line Height (display only)
                    .child(
                        setting_row(
                            &theme,
                            "Line Height",
                            div().text_color(theme.text).child(line_height_val),
                        ),
                    )
                    // Copy Full Precision (toggle)
                    .child(
                        setting_row(
                            &theme,
                            "Copy Full Precision",
                            div()
                                .id("copy-fp-toggle")
                                .cursor_pointer()
                                .text_color(theme.text)
                                .hover(|s| s.text_color(theme.text_muted))
                                .child(copy_fp_val.to_string())
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                        this.toggle_copy_full_precision(cx);
                                    }),
                                ),
                        ),
                    )
                    // Tab Size (+/-)
                    .child(
                        setting_row(
                            &theme,
                            "Tab Size",
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .id("ts-dec")
                                        .cursor_pointer()
                                        .text_color(theme.text_muted)
                                        .hover(|s| s.text_color(theme.text))
                                        .child("-")
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                                this.dec_tab_size(cx);
                                            }),
                                        ),
                                )
                                .child(div().text_color(theme.text).child(tab_size_val))
                                .child(
                                    div()
                                        .id("ts-inc")
                                        .cursor_pointer()
                                        .text_color(theme.text_muted)
                                        .hover(|s| s.text_color(theme.text))
                                        .child("+")
                                        .on_mouse_up(
                                            MouseButton::Left,
                                            cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                                this.inc_tab_size(cx);
                                            }),
                                        ),
                                ),
                        ),
                    ),
            )
            .into_any_element()
    }
}

fn setting_row(
    theme: &Theme,
    label: &str,
    value_el: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .w_full()
        .py(px(6.))
        .justify_between()
        .items_center()
        .child(div().text_color(theme.text_muted).child(label.to_string()))
        .child(value_el)
}
