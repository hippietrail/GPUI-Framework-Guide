use std::collections::HashMap;

use gpui::{Context, Entity, Render, SharedString, Window, div, prelude::*, px};
use numnum_core::format::{format_value, format_value_full_precision};
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
        copy_full_precision: bool,
        live_rates: HashMap<String, f64>,
    ) -> Self {
        let results_pane = cx.new(|_| ResultsPane::new(theme.clone(), copy_full_precision));
        let status_bar = cx.new(|_| StatusBar::new(theme.clone()));

        let theme_clone = theme.clone();
        let font_for_app = font_family.clone();

        let editor = cx.new(|cx| {
            Editor::new(cx, theme_clone, font_family, font_size, None)
        });

        // Observe editor for content changes: evaluate, update results + diagnostics
        let results_for_eval = results_pane.clone();
        let status_for_eval = status_bar.clone();
        let editor_for_eval = editor.clone();
        cx.observe(&editor, move |_this, _editor_entity, cx| {
            let content = editor_for_eval.read(cx).content().to_string();
            let (line, col) = editor_for_eval.read(cx).cursor_line_col();

            let mut eval_ctx = EvalContext::new();
            rates::apply_rates(&mut eval_ctx.currency_table, &live_rates);
            let mut results = Vec::new();
            let mut diagnostics: Vec<Option<String>> = Vec::new();
            let mut running_total = Value::None;

            for line_text in content.split('\n') {
                match eval_ctx.eval_line(line_text) {
                    Ok(Value::None) => {
                        results.push(LineResult::None);
                        diagnostics.push(None);
                    }
                    Ok(val) => {
                        let formatted = format_value(
                            &val, &eval_ctx.unit_table, &eval_ctx.currency_table,
                        );
                        // Full precision for clipboard copy
                        let full_precision = format_value_full_precision(
                            &val, &eval_ctx.unit_table, &eval_ctx.currency_table,
                        );
                        if let Some(n) = val.as_number() {
                            match &running_total {
                                Value::None => running_total = Value::Number(n),
                                Value::Number(prev) => running_total = Value::Number(prev + n),
                                _ => running_total = Value::Number(n),
                            }
                        }
                        results.push(LineResult::Value(formatted, full_precision));
                        diagnostics.push(None);
                    }
                    Err(e) => {
                        results.push(LineResult::None);
                        diagnostics.push(Some(e.to_string()));
                    }
                }
            }

            let total_str = match &running_total {
                Value::Number(n) => numnum_core::format::format_number(*n),
                _ => String::new(),
            };

            // Update editor diagnostics (for inlay rendering)
            editor_for_eval.update(cx, |editor, _cx| {
                editor.diagnostics = diagnostics.clone();
            });

            results_for_eval.update(cx, |pane, cx| {
                pane.set_results_with_diagnostics(results, &diagnostics, cx);
            });
            status_for_eval.update(cx, |bar, cx| {
                bar.set_running_total(total_str.clone(), cx);
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
