use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gpui::{
    App, Context, CursorStyle, Entity, Focusable, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, SharedString,
    Window, WindowControlArea, div, point, prelude::*, px,
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
    was_settings_visible: bool,
    appearance_mode: String,
    dark_theme_name: String,
    light_theme_name: String,
    show_diagnostics: bool,
    viewport_height: Pixels,
    autoscroll_to_line: Option<usize>,
    title_bar: String,
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
        let appearance_mode = settings.appearance.mode.clone();
        let show_diagnostics = settings.editor.show_diagnostics;

        let results_pane = cx.new(|_| ResultsPane::new(theme.clone(), copy_full_precision));

        // Create settings pane
        let settings_pane = cx.new(|cx| SettingsPane::new(cx, settings.clone(), theme.clone()));

        // Create status bar with settings callback
        let settings_pane_for_bar = settings_pane.clone();
        let status_bar = cx.new(|_| StatusBar::new(
            theme.clone(),
            Some(Arc::new(move |_window: &mut Window, cx: &mut App| {
                settings_pane_for_bar.update(cx, |pane, cx| {
                    pane.toggle(cx);
                });
            })),
        ));

        let theme_clone = theme.clone();
        let font_for_app = font_family.clone();

        // Build tables once — shared by editor (highlighting/autocomplete) and eval observer
        let unit_table = numnum_core::types::UnitTable::new();
        let currency_table = numnum_core::types::CurrencyTable::new();

        let unit_table_for_eval = unit_table.clone();
        let currency_table_for_eval = currency_table.clone();

        let editor = cx.new(|cx| {
            let mut ed = Editor::new(cx, theme_clone, font_family, font_size, None,
                unit_table, currency_table);
            ed.show_diagnostics = show_diagnostics;
            ed
        });

        let scroll_handle = ScrollHandle::new();
        let last_scroll_line = std::cell::Cell::new(0usize);

        // Observe editor for content changes: evaluate, update results + diagnostics
        let results_for_eval = results_pane.clone();
        let status_for_eval = status_bar.clone();
        let editor_for_eval = editor.clone();
        cx.observe(&editor, move |this, _editor_entity, cx| {
            let content = editor_for_eval.read(cx).content().to_string();
            let (line, col) = editor_for_eval.read(cx).cursor_line_col();
            let precision = this.precision;

            let mut eval_ctx = EvalContext::with_tables(
                unit_table_for_eval.clone(),
                currency_table_for_eval.clone(),
            );
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

            // Update editor diagnostics and variables (for inlay rendering + autocomplete)
            let visual_counts = editor_for_eval.read(cx).line_visual_counts.clone();
            let show_diags = this.show_diagnostics;
            let var_names: Vec<String> = eval_ctx.variables.keys().cloned().collect();
            editor_for_eval.update(cx, |editor, _cx| {
                if show_diags {
                    editor.diagnostics = diagnostics.clone();
                } else {
                    editor.diagnostics = vec![None; diagnostics.len()];
                }
                editor.set_known_variables(var_names);
            });

            results_for_eval.update(cx, |pane, cx| {
                pane.line_visual_counts = visual_counts;
                pane.set_results_with_diagnostics(results, &diagnostics, cx);
            });
            status_for_eval.update(cx, |bar, cx| {
                bar.set_running_total(total_str.clone(), cx);
                bar.set_cursor(line, col, cx);
            });

            // Request auto-scroll — actual scroll happens in render() with real layout data
            if line != last_scroll_line.get() {
                last_scroll_line.set(line);
                this.autoscroll_to_line = Some(line);
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
            this.font_family = SharedString::from(new_settings.editor.font_family.clone());
            this.show_diagnostics = new_settings.editor.show_diagnostics;

            // Reload theme if appearance mode or theme selection changed
            let new_mode = new_settings.appearance.mode.clone();
            let new_dark = new_settings.appearance.dark_theme.clone();
            let new_light = new_settings.appearance.light_theme.clone();
            let mode_changed = new_mode != this.appearance_mode;
            let dark_changed = new_dark != this.dark_theme_name;
            let light_changed = new_light != this.light_theme_name;

            if mode_changed || dark_changed || light_changed {
                this.appearance_mode = new_mode.clone();
                this.dark_theme_name = new_dark.clone();
                this.light_theme_name = new_light.clone();

                let theme_name = match new_mode.as_str() {
                    "dark" => new_dark,
                    "light" => new_light,
                    _ => new_dark, // auto defaults to dark
                };
                let tf = numnum_core::ThemeFile::load(&theme_name);
                let new_theme = crate::theme::Theme::from_theme_file(&tf);
                this.apply_theme(new_theme, cx);
            }

            // Update results pane copy_full_precision
            let copy_fp = new_settings.editor.copy_full_precision;
            results_pane_for_save.update(cx, |rp, cx| {
                rp.copy_full_precision = copy_fp;
                cx.notify();
            });

            // Propagate font changes to editor
            let new_font_size = new_settings.editor.font_size;
            let new_font_family = new_settings.editor.font_family.clone();
            this.editor.update(cx, |editor, _| {
                editor.font_size = px(new_font_size);
                editor.font_family = SharedString::from(new_font_family);
            });

            // Re-scroll to cursor with new line height
            let (line, _) = this.editor.read(cx).cursor_line_col();
            this.autoscroll_to_line = Some(line);
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
            was_settings_visible: false,
            scroll_handle,
            appearance_mode,
            dark_theme_name: settings.appearance.dark_theme.clone(),
            light_theme_name: settings.appearance.light_theme.clone(),
            show_diagnostics,
            viewport_height: px(0.),
            autoscroll_to_line: None,
            title_bar: settings.window.title_bar.clone(),
        }
    }
}

impl NumNumApp {
    pub fn apply_theme(&mut self, new_theme: crate::theme::Theme, cx: &mut Context<Self>) {
        self.theme = new_theme.clone();
        self.editor.update(cx, |ed, _| { ed.theme = new_theme.clone(); });
        self.results_pane.update(cx, |rp, _| { rp.theme = new_theme.clone(); });
        self.status_bar.update(cx, |sb, _| { sb.theme = new_theme.clone(); });
        self.settings_pane.update(cx, |sp, _| { sp.theme = new_theme.clone(); });
        cx.notify();
    }

    fn on_ctrl_scroll(&mut self, event: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !event.modifiers.control { return; }
        let delta_y = match event.delta {
            ScrollDelta::Lines(pt) => pt.y,
            ScrollDelta::Pixels(pt) => {
                let px_val: f32 = pt.y.into();
                px_val / 20.0 // normalize pixel delta to line-like units
            }
        };
        if delta_y.abs() < 0.01 { return; }
        let step = if delta_y > 0.0 { -1.0 } else { 1.0 };
        self.font_size = (self.font_size + step).clamp(8.0, 72.0);
        window.set_rem_size(px(self.font_size));
        self.editor.update(cx, |editor, _| {
            editor.font_size = px(self.font_size);
        });
        // Sync to settings pane + persist
        self.settings_pane.update(cx, |sp, cx| {
            sp.update_font_size(self.font_size, cx);
        });
        // Re-scroll to cursor with new line height
        let (line, _) = self.editor.read(cx).cursor_line_col();
        self.autoscroll_to_line = Some(line);
        cx.notify();
    }

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
            let viewport_width: f32 = window.viewport_size().width.into();
            if viewport_width > 0.0 {
                let mouse_x: f32 = event.position.x.into();
                let ratio = mouse_x / viewport_width;
                self.split_ratio = ratio.clamp(0.3, 0.85);
                cx.notify();
            }
        }
    }
}

impl Render for NumNumApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.viewport_height = window.viewport_size().height;
        // Sync rem_size when font_size changed (settings observer can't access window)
        let current_rem: f32 = window.rem_size().into();
        if (current_rem - self.font_size).abs() > 0.01 {
            window.set_rem_size(px(self.font_size));
        }
        let settings_visible = self.settings_pane.read(cx).visible;

        // Refocus editor when settings closes
        if self.was_settings_visible && !settings_visible {
            let focus = self.editor.focus_handle(cx);
            window.focus(&focus, cx);
            // Reset cursor blink so it's immediately visible
            self.editor.update(cx, |editor, cx| {
                editor.pause_blinking(cx);
            });
        }
        self.was_settings_visible = settings_visible;

        let divider_color = self.theme.divider;
        let is_dragging = self.is_dragging_divider;

        // Auto-scroll to cursor using real layout data
        let line_height = window.line_height();
        // Scrollable viewport = window height - status bar (28px) - custom titlebar if present
        let titlebar_height = if self.title_bar == "numnum" { px(32.) } else { px(0.) };
        let scroll_viewport_height = self.viewport_height - px(28.) - titlebar_height;
        if let Some(target_line) = self.autoscroll_to_line.take() {
            let visual_counts = &self.editor.read(cx).line_visual_counts;
            if !visual_counts.is_empty() && scroll_viewport_height > px(0.) {
                let cursor_y: Pixels = visual_counts.iter().take(target_line)
                    .map(|&c| line_height * c as f32)
                    .sum();

                let current_offset = self.scroll_handle.offset();
                let viewport_top = -current_offset.y;
                let viewport_bottom = viewport_top + scroll_viewport_height;

                if cursor_y + line_height > viewport_bottom {
                    let new_y = -(cursor_y - scroll_viewport_height + line_height * 3.0);
                    self.scroll_handle.set_offset(point(px(0.), new_y));
                } else if cursor_y < viewport_top {
                    let new_y = -cursor_y + line_height;
                    self.scroll_handle.set_offset(point(px(0.), new_y));
                }
            }
        }

        // Compute content height from editor visual line counts + diagnostics
        let editor = self.editor.read(cx);
        let total_visual: usize = if editor.line_visual_counts.is_empty() {
            editor.content().split('\n').count()
        } else {
            editor.line_visual_counts.iter().sum()
        };
        let diag_count = editor.diagnostics.iter().filter(|d| d.is_some()).count();
        let diag_line_height = line_height * 0.8; // proportional to line height
        let content_height = line_height * (total_visual as f32) + diag_line_height * (diag_count as f32) + line_height * 4.0; // padding at bottom

        let split_ratio = self.split_ratio;
        let scroll_handle = self.scroll_handle.clone();
        let editor_entity = self.editor.clone();
        let results_entity = self.results_pane.clone();
        let bg = self.theme.background;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(self.theme.background)
            .text_color(self.theme.text)
            .font_family(self.font_family.clone())
            .text_size(px(self.font_size))
            .on_scroll_wheel(cx.listener(Self::on_ctrl_scroll))
            // Custom "numnum" title bar
            .when(self.title_bar == "numnum", |el| {
                let close_color = self.theme.error;
                let minimize_color = self.theme.syn_variable;
                let maximize_color = self.theme.result;
                let title_color = self.theme.text_muted;
                let bar_bg = self.theme.status_bar;

                el.child(
                    div()
                        .id("numnum-titlebar")
                        .w_full()
                        .h(px(32.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .bg(bar_bg)
                        .window_control_area(WindowControlArea::Drag)
                        // Traffic light buttons (left side)
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(8.))
                                .pl(px(12.))
                                .child(
                                    div()
                                        .id("tb-close")
                                        .size(px(12.))
                                        .rounded_full()
                                        .bg(close_color)
                                        .hover(|s| s.bg(close_color))
                                        .cursor(CursorStyle::PointingHand)
                                        .window_control_area(WindowControlArea::Close),
                                )
                                .child(
                                    div()
                                        .id("tb-min")
                                        .size(px(12.))
                                        .rounded_full()
                                        .bg(minimize_color)
                                        .hover(|s| s.bg(minimize_color))
                                        .cursor(CursorStyle::PointingHand)
                                        .window_control_area(WindowControlArea::Min),
                                )
                                .child(
                                    div()
                                        .id("tb-max")
                                        .size(px(12.))
                                        .rounded_full()
                                        .bg(maximize_color)
                                        .hover(|s| s.bg(maximize_color))
                                        .cursor(CursorStyle::PointingHand)
                                        .window_control_area(WindowControlArea::Max),
                                ),
                        )
                        // Centered title
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .justify_center()
                                .text_size(px(13.))
                                .text_color(title_color)
                                .child("NumNum"),
                        )
                        // Right spacer (same width as traffic lights for centering)
                        .child(div().w(px(68.))),
                )
            })
            // Only attach divider drag handlers when calculator is showing
            .when(!settings_visible, |el| {
                el.on_mouse_move(cx.listener(Self::on_divider_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_divider_up))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_divider_up))
            })
            // Main area: either calculator or settings
            .when(!settings_visible, |el| {
                el.child(
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
                                .track_scroll(&scroll_handle)
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
                                                .flex_basis(relative(split_ratio))
                                                .min_w_0()
                                                .child(editor_entity),
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
                                                .flex_basis(relative(1.0 - split_ratio))
                                                .min_w_0()
                                                .overflow_x_hidden()
                                                .bg(bg)
                                                .child(results_entity),
                                        ),
                                ),
                        )
                        .child(
                            // Divider — OUTSIDE scroll, full viewport height, positioned over the spacer
                            div()
                                .id("split-divider")
                                .group("divider")
                                .absolute()
                                .left(relative(split_ratio))
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
            })
            .when(settings_visible, |el| {
                el.child(self.settings_pane.clone())
            })
            // Status bar always visible
            .child(self.status_bar.clone())
    }
}
