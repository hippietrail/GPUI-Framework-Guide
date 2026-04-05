use std::collections::HashMap;

use gpui::{
    Context, CursorStyle, Entity, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Render, SharedString, Window, div, prelude::*, px,
};
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
    split_ratio: f32, // 0.0-1.0, fraction of width for editor
    is_dragging_divider: bool,
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
            let mut running_total: f64 = 0.0;
            let mut last_val = Value::None;
            let mut has_any_value = false;

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
                        let full_precision = format_value_full_precision(
                            &val, &eval_ctx.unit_table, &eval_ctx.currency_table,
                        );
                        if let Some(n) = val.as_number() {
                            running_total += n;
                            last_val = val.clone();
                            has_any_value = true;
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

            // Format total with the last result's unit/currency
            let total_str = if !has_any_value {
                String::new()
            } else {
                let total_val = match &last_val {
                    Value::WithUnit(_, u) => Value::WithUnit(running_total, *u),
                    Value::WithCurrency(_, c) => Value::WithCurrency(running_total, *c),
                    _ => Value::Number(running_total),
                };
                format_value(&total_val, &eval_ctx.unit_table, &eval_ctx.currency_table)
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
            split_ratio: 0.7,
            is_dragging_divider: false,
        }
    }
}

impl NumNumApp {
    fn on_divider_down(&mut self, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_dragging_divider = true;
        cx.notify();
    }

    fn on_divider_up(&mut self, _: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_dragging_divider = false;
        cx.notify();
    }

    fn on_divider_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_dragging_divider {
            let bounds = window.bounds();
            let window_width: f32 = bounds.size.width.into();
            if window_width > 0.0 {
                let mouse_x: f32 = event.position.x.into();
                let origin_x: f32 = bounds.origin.x.into();
                let ratio = (mouse_x - origin_x) / window_width;
                self.split_ratio = ratio.clamp(0.3, 0.85);
                cx.notify();
            }
        }
    }
}

impl Render for NumNumApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let divider_color = self.theme.divider;
        let is_dragging = self.is_dragging_divider;

        // Compute content height from editor line count + diagnostics
        let editor = self.editor.read(cx);
        let line_count = editor.content().split('\n').count();
        let diag_count = editor.diagnostics.iter().filter(|d| d.is_some()).count();
        let line_height = window.line_height();
        let diag_line_height = px(20.0);
        let content_height = line_height * (line_count as f32) + diag_line_height * (diag_count as f32) + px(100.0); // extra padding at bottom

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(self.theme.background)
            .text_color(self.theme.text)
            .font_family(self.font_family.clone())
            .text_size(px(self.font_size))
            .on_mouse_move(cx.listener(Self::on_divider_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_divider_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_divider_up))
            .child(
                // Scroll viewport
                div()
                    .id("scroll-viewport")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        // Fixed-height content (taller than viewport to enable scroll)
                        div()
                            .w_full()
                            .h(content_height)
                            .flex()
                            .flex_row()
                            .child(
                                // Editor pane
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(self.editor.clone()),
                            )
                            .child(
                                // Divider — visible on hover only
                                div()
                                    .id("split-divider")
                                    .group("divider")
                                    .w(px(14.))
                                    .flex_shrink_0()
                                    .flex()
                                    .justify_center()
                                    .cursor(CursorStyle::ResizeLeftRight)
                                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_divider_down))
                                    .child(
                                        div()
                                            .w(px(5.))
                                            .h_full()
                                            .rounded_sm()
                                            .when(is_dragging, |el| el.bg(divider_color))
                                            .group_hover("divider", |style| style.bg(divider_color)),
                                    ),
                            )
                            .child(
                                // Results pane
                                div()
                                    .w(px((1.0 - self.split_ratio) * 900.0))
                                    .flex_shrink_0()
                                    .bg(self.theme.background)
                                    .child(self.results_pane.clone()),
                            ),
                    ),
            )
            .child(self.status_bar.clone())
    }
}
