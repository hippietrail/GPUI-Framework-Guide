use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gpui::{
    App, Context, CursorStyle, Entity, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Render, ScrollHandle, SharedString, Window, div, point, prelude::*, px,
};

use gpui::relative;
use numnum_core::format::{format_value_with_precision, format_value_full_precision};
use numnum_core::{EvalContext, Settings, Value};

use crate::editor::Editor;
use crate::rates;
use crate::results_pane::{LineResult, ResultsPane};
use crate::settings_pane::SettingsPane;
use crate::status_bar::StatusBar;
use crate::theme::Theme;

pub struct NumNumApp {
    pub editor: Entity<Editor>,
    results_pane: Entity<ResultsPane>,
    status_bar: Entity<StatusBar>,
    settings_pane: Entity<SettingsPane>,
    theme: Theme,
    font_family: SharedString,
    font_size: f32,
    precision: u32,
    split_ratio: f32, // 0.0-1.0, fraction of width for editor
    is_dragging_divider: bool,
    scroll_handle: ScrollHandle,
}

impl NumNumApp {
    pub fn new(
        cx: &mut Context<Self>,
        theme: Theme,
        settings: Settings,
        live_rates: Arc<Mutex<HashMap<String, f64>>>,
    ) -> Self {
        let font_family = settings.editor.font_family.clone();
        let font_size = settings.editor.font_size;
        let copy_full_precision = settings.editor.copy_full_precision;
        let precision = settings.editor.precision;

        let results_pane = cx.new(|_| ResultsPane::new(theme.clone(), copy_full_precision));

        // Create settings pane
        let settings_pane = cx.new(|_| SettingsPane::new(settings.clone(), theme.clone()));

        // Create status bar with settings callback
        let settings_pane_for_bar = settings_pane.clone();
        let status_bar = cx.new(|_| StatusBar::new(
            theme.clone(),
            Some(Box::new(move |_window: &mut Window, cx: &mut App| {
                settings_pane_for_bar.update(cx, |pane, cx| {
                    pane.toggle(cx);
                });
            })),
        ));

        let theme_clone = theme.clone();
        let font_for_app = font_family.clone();

        let editor = cx.new(|cx| {
            Editor::new(cx, theme_clone, font_family, font_size, None)
        });

        // Create scroll handle shared between observer and render
        let scroll_handle = ScrollHandle::new();
        let scroll_handle_for_eval = scroll_handle.clone();
        let last_scroll_line = std::cell::Cell::new(0usize);

        // Observe editor for content changes: evaluate, update results + diagnostics
        let results_for_eval = results_pane.clone();
        let status_for_eval = status_bar.clone();
        let editor_for_eval = editor.clone();
        cx.observe(&editor, move |this, _editor_entity, cx| {
            let content = editor_for_eval.read(cx).content().to_string();
            let (line, col) = editor_for_eval.read(cx).cursor_line_col();
            let precision = this.precision;

            let mut eval_ctx = EvalContext::new();
            if let Ok(rates) = live_rates.lock() {
                rates::apply_rates(&mut eval_ctx.currency_table, &rates);
            }
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
                        let formatted = format_value_with_precision(
                            &val, &eval_ctx.unit_table, &eval_ctx.currency_table, precision,
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
                format_value_with_precision(&total_val, &eval_ctx.unit_table, &eval_ctx.currency_table, precision)
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

            // Auto-scroll to keep cursor visible, but only when cursor line changes
            if line != last_scroll_line.get() {
                last_scroll_line.set(line);
                let approx_line_height = px(font_size * 1.6);
                let cursor_y = approx_line_height * (line as f32);
                let current_offset = scroll_handle_for_eval.offset();
                let viewport_top = -current_offset.y;
                // Infer viewport height from scroll handle bounds
                let max_offset = scroll_handle_for_eval.max_offset();
                let max_y: f32 = max_offset.y.into();
                // max_offset.y is the most negative the scroll can go
                // viewport_height ≈ content_height + max_y (since max_y is negative)
                let total_lines = editor_for_eval.read(cx).content().split('\n').count();
                let content_h: f32 = (approx_line_height * (total_lines as f32 + 4.0)).into();
                let viewport_height = if max_y < 0.0 {
                    px(content_h + max_y) // content - scrollable range = viewport
                } else {
                    px(content_h.min(600.0)) // fallback
                };
                let viewport_bottom = viewport_top + viewport_height;

                if cursor_y + approx_line_height > viewport_bottom {
                    let new_y = -(cursor_y - viewport_height + approx_line_height * 3.0);
                    scroll_handle_for_eval.set_offset(point(px(0.), new_y));
                } else if cursor_y < viewport_top {
                    let new_y = -cursor_y + approx_line_height;
                    scroll_handle_for_eval.set_offset(point(px(0.), new_y));
                }
            }
        })
        .detach();

        // Wire up on_save callback: settings pane -> app
        let results_pane_for_save = results_pane.clone();
        let settings_pane_clone = settings_pane.clone();
        cx.observe(&settings_pane_clone, move |this, settings_entity, cx| {
            let pane = settings_entity.read(cx);
            let new_settings = pane.current_settings();
            this.precision = new_settings.editor.precision;
            this.font_size = new_settings.editor.font_size;
            // Update results pane copy_full_precision
            let copy_fp = new_settings.editor.copy_full_precision;
            results_pane_for_save.update(cx, |rp, cx| {
                rp.copy_full_precision = copy_fp;
                cx.notify();
            });
            cx.notify();
        }).detach();

        NumNumApp {
            editor,
            results_pane,
            status_bar,
            settings_pane,
            theme,
            font_family: SharedString::from(font_for_app),
            font_size,
            precision,
            split_ratio: 0.7,
            is_dragging_divider: false,
            scroll_handle,
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
                // Main area: flex_row with scroll viewport + fixed divider
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(
                        // Scroll viewport (editor + results scroll together)
                        div()
                            .id("scroll-viewport")
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .child(
                                // Fixed-height content
                                div()
                                    .w_full()
                                    .h(content_height)
                                    .flex()
                                    .flex_row()
                                    .child(
                                        // Editor pane — proportional width
                                        div()
                                            .flex_grow()
                                            .flex_shrink()
                                            .flex_basis(relative(self.split_ratio))
                                            .min_w_0()
                                            .child(self.editor.clone()),
                                    )
                                    .child(
                                        // Spacer for divider width
                                        div().w(px(14.)).flex_shrink_0(),
                                    )
                                    .child(
                                        // Results pane — proportional width
                                        div()
                                            .flex_grow()
                                            .flex_shrink()
                                            .flex_basis(relative(1.0 - self.split_ratio))
                                            .min_w_0()
                                            .bg(self.theme.background)
                                            .child(self.results_pane.clone()),
                                    ),
                            ),
                    )
                    .child(
                        // Divider — OUTSIDE scroll, full viewport height, positioned over the spacer
                        div()
                            .id("split-divider")
                            .group("divider")
                            .absolute()
                            .left(relative(self.split_ratio))
                            .top_0()
                            .bottom_0()
                            .w(px(14.))
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
                    ),
            )
            .child(self.status_bar.clone())
            .child(self.settings_pane.clone())
    }
}
