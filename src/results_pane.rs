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
    has_diagnostic: Vec<bool>,
    theme: Theme,
}

impl ResultsPane {
    pub fn new(theme: Theme) -> Self {
        ResultsPane {
            results: Vec::new(),
            has_diagnostic: Vec::new(),
            theme,
        }
    }

    pub fn set_results_with_diagnostics(
        &mut self,
        results: Vec<LineResult>,
        diagnostics: &[Option<String>],
        cx: &mut Context<Self>,
    ) {
        self.has_diagnostic = diagnostics.iter().map(|d| d.is_some()).collect();
        self.results = results;
        cx.notify();
    }
}

impl Render for ResultsPane {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let line_height = window.line_height();
        let theme = self.theme.clone();
        let results = self.results.clone();

        let has_diag = self.has_diagnostic.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .overflow_y_hidden()
            .children({
                let mut children: Vec<gpui::AnyElement> = Vec::new();
                for (i, result) in results.into_iter().enumerate() {
                    let (text, color) = match &result {
                        LineResult::None => (String::new(), theme.text_dimmed),
                        LineResult::Value(s) => (s.clone(), theme.result),
                        LineResult::Error(s) => (s.clone(), theme.error),
                    };

                    let text_for_copy = text.clone();
                    children.push(
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
                            .into_any_element(),
                    );

                    // Insert empty spacer if this line has a diagnostic inlay
                    if has_diag.get(i).copied().unwrap_or(false) {
                        children.push(
                            div()
                                .w_full()
                                .h(px(18.)) // approximate height of diagnostic text
                                .into_any_element(),
                        );
                    }
                }
                children
            })
    }
}
