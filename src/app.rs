use std::collections::HashMap;

use gpui::{App, Context, Entity, Render, SharedString, Window, div, prelude::*, px};
use numnum_core::format::format_value;
use numnum_core::{EvalContext, Value};

use crate::editor::Editor;
use crate::rates;
use crate::results_pane::{LineResult, ResultsPane};
use crate::status_bar::StatusBar;
use crate::theme::Theme;

pub struct NumNumApp {
    pub editor: Entity<Editor>,
    results_pane: Entity<ResultsPane>,
    status_bar: Entity<StatusBar>,
    theme: Theme,
    font_family: SharedString,
    font_size: f32,
}

impl NumNumApp {
    pub fn new(
        cx: &mut Context<Self>,
        theme: Theme,
        font_family: String,
        font_size: f32,
        live_rates: HashMap<String, f64>,
    ) -> Self {
        let results_pane = cx.new(|_| ResultsPane::new(theme.clone()));
        let status_bar = cx.new(|_| StatusBar::new(theme.clone()));

        let results_entity = results_pane.clone();
        let status_entity = status_bar.clone();
        let theme_clone = theme.clone();
        let font_for_app = font_family.clone();

        let editor = cx.new(|cx| {
            Editor::new(
                cx,
                theme_clone,
                font_family,
                font_size,
                Some(Box::new(move |content: &str, _window: &mut Window, cx: &mut App| {
                    // Evaluate all lines
                    let mut eval_ctx = EvalContext::new();
                    rates::apply_rates(&mut eval_ctx.currency_table, &live_rates);
                    let mut results = Vec::new();
                    let mut running_total = Value::None;

                    for line in content.split('\n') {
                        match eval_ctx.eval_line(line) {
                            Ok(Value::None) => {
                                results.push(LineResult::None);
                            }
                            Ok(val) => {
                                let formatted = format_value(
                                    &val,
                                    &eval_ctx.unit_table,
                                    &eval_ctx.currency_table,
                                );
                                // Track running total
                                if let Some(n) = val.as_number() {
                                    match &running_total {
                                        Value::None => running_total = Value::Number(n),
                                        Value::Number(prev) => {
                                            running_total = Value::Number(prev + n)
                                        }
                                        _ => running_total = Value::Number(n),
                                    }
                                }
                                results.push(LineResult::Value(formatted));
                            }
                            Err(e) => {
                                results.push(LineResult::Error(e.to_string()));
                            }
                        }
                    }

                    let total_str = match &running_total {
                        Value::Number(n) => numnum_core::format::format_number(*n),
                        _ => String::new(),
                    };

                    results_entity.update(cx, |pane, cx| {
                        pane.set_results(results, cx);
                    });
                    status_entity.update(cx, |bar, cx| {
                        bar.set_running_total(total_str, cx);
                    });
                })),
            )
        });

        // Observe editor for cursor position updates
        let status_for_observe = status_bar.clone();
        let editor_for_observe = editor.clone();
        cx.observe(&editor, move |_this, _editor_entity, cx| {
            let editor_read = editor_for_observe.read(cx);
            let (line, col) = editor_read.cursor_line_col();
            status_for_observe.update(cx, |bar, cx| {
                bar.set_cursor(line, col, cx);
            });
        })
        .detach();

        NumNumApp {
            editor,
            results_pane,
            status_bar,
            theme,
            font_family: SharedString::from(font_for_app),
            font_size,
        }
    }
}

impl Render for NumNumApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(self.theme.background)
            .text_color(self.theme.text)
            .font_family(self.font_family.clone())
            .text_size(px(self.font_size))
            .child(
                // Main content area: editor | divider | results
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(
                        // Editor pane
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .child(self.editor.clone()),
                    )
                    .child(
                        // Divider
                        div()
                            .w(px(1.))
                            .h_full()
                            .bg(self.theme.divider)
                            .flex_shrink_0(),
                    )
                    .child(
                        // Results pane
                        div()
                            .w(px(200.))
                            .flex_shrink_0()
                            .overflow_hidden()
                            .bg(self.theme.background)
                            .child(self.results_pane.clone()),
                    ),
            )
            .child(
                // Divider above status bar
                div()
                    .w_full()
                    .h(px(1.))
                    .bg(self.theme.divider)
                    .flex_shrink_0(),
            )
            .child(self.status_bar.clone())
    }
}
