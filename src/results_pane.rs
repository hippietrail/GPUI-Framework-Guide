use gpui::{
    App, ClipboardItem, Context, MouseButton, MouseUpEvent, Render, Window, div, prelude::*, px,
};

use crate::theme::Theme;

#[derive(Clone, Debug)]
pub enum LineResult {
    None,
    Value(String),
    Error(String),
}

pub struct ResultsPane {
    results: Vec<LineResult>,
    theme: Theme,
}

impl ResultsPane {
    pub fn new(theme: Theme) -> Self {
        ResultsPane {
            results: Vec::new(),
            theme,
        }
    }

    pub fn set_results(&mut self, results: Vec<LineResult>, cx: &mut Context<Self>) {
        self.results = results;
        cx.notify();
    }
}

impl Render for ResultsPane {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let line_height = window.line_height();
        let theme = self.theme.clone();
        let results = self.results.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .overflow_y_hidden()
            .children(results.into_iter().enumerate().map(|(_i, result)| {
                let (text, color) = match &result {
                    LineResult::None => (String::new(), theme.text_dimmed),
                    LineResult::Value(s) => (s.clone(), theme.result),
                    LineResult::Error(s) => (s.clone(), theme.error),
                };

                let text_for_copy = text.clone();
                div()
                    .h(line_height)
                    .w_full()
                    .flex()
                    .items_end()
                    .justify_end()
                    .px(px(8.))
                    .text_color(color)
                    .when(!text_for_copy.is_empty(), |el| {
                        el.cursor_pointer()
                            .on_mouse_up(
                                MouseButton::Left,
                                move |_: &MouseUpEvent, _window: &mut Window, cx: &mut App| {
                                    if !text_for_copy.is_empty() {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            text_for_copy.clone(),
                                        ));
                                    }
                                },
                            )
                    })
                    .child(text)
            }))
    }
}
