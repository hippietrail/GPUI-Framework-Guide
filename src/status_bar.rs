use std::sync::Arc;

use gpui::{App, Context, MouseButton, MouseUpEvent, Render, Window, div, prelude::*, px, svg};

use crate::theme::Theme;

pub struct StatusBar {
    line: usize,
    col: usize,
    running_total: String,
    theme: Theme,
    on_settings_click: Option<Arc<dyn Fn(&mut Window, &mut App)>>,
}

impl StatusBar {
    pub fn new(
        theme: Theme,
        on_settings_click: Option<Arc<dyn Fn(&mut Window, &mut App)>>,
    ) -> Self {
        StatusBar {
            line: 1,
            col: 1,
            running_total: String::new(),
            theme,
            on_settings_click,
        }
    }

    pub fn set_cursor(&mut self, line: usize, col: usize, cx: &mut Context<Self>) {
        self.line = line + 1; // 1-indexed display
        self.col = col + 1;
        cx.notify();
    }

    pub fn set_running_total(&mut self, total: String, cx: &mut Context<Self>) {
        self.running_total = total;
        cx.notify();
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let icon_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/settings.svg");

        let on_settings = self.on_settings_click.clone();
        let icon_color = self.theme.text_muted;

        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(28.))
            .flex_shrink_0()
            .bg(self.theme.background)
            .px(px(12.))
            .items_center()
            .text_size(px(12.))
            // Left: Settings gear icon
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(80.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id("settings-btn")
                            .cursor_pointer()
                            .child(
                                svg()
                                    .external_path(icon_path)
                                    .size(px(16.))
                                    .text_color(icon_color)
                            )
                            .hover(|s| s.opacity(0.7))
                            .when_some(on_settings, |el, cb| {
                                el.on_mouse_up(
                                    MouseButton::Left,
                                    move |_: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                                        cb(window, cx);
                                    },
                                )
                            })
                    ),
            )
            // Center: total
            .child(
                div()
                    .flex_1()
                    .text_color(self.theme.text_muted)
                    .flex()
                    .justify_center()
                    .child(if self.running_total.is_empty() {
                        String::new()
                    } else {
                        format!("Total: {}", self.running_total)
                    }),
            )
            // Right: Ln X, Col Y
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(80.))
                    .flex()
                    .justify_end()
                    .text_color(self.theme.text_dimmed)
                    .child(format!("Ln {}, Col {}", self.line, self.col)),
            )
    }
}
