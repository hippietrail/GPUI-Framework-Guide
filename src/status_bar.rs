use gpui::{Context, Render, Window, div, prelude::*, px};

use crate::theme::Theme;

pub struct StatusBar {
    line: usize,
    col: usize,
    running_total: String,
    theme: Theme,
}

impl StatusBar {
    pub fn new(theme: Theme) -> Self {
        StatusBar {
            line: 1,
            col: 1,
            running_total: String::new(),
            theme,
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
        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(28.))
            .flex_shrink_0()
            .bg(self.theme.status_bar)
            .px(px(12.))
            .items_center()
            .justify_between()
            .text_size(px(12.))
            .child(
                div()
                    .text_color(self.theme.text_muted)
                    .child(if self.running_total.is_empty() {
                        String::new()
                    } else {
                        format!("Total: {}", self.running_total)
                    }),
            )
            .child(
                div()
                    .text_color(self.theme.text_dimmed)
                    .child(format!("Ln {}, Col {}", self.line, self.col)),
            )
    }
}
