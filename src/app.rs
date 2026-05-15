use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{
    App, ClickEvent, Context, CursorStyle, Entity, Focusable, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent,
    SharedString, Window, div, point, prelude::*, px, svg,
};

use gpui::relative;
use numnum_core::format::{format_value_with_precision, format_value_full_precision};
use numnum_core::types::NumberFormat;
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
    titlebar_should_move: bool,
    number_format: NumberFormat,
    session_path: Option<PathBuf>,
    session_list: Vec<(PathBuf, crate::session::Session)>,
    burger_menu_open: bool,
}

impl NumNumApp {
    pub fn new(
        cx: &mut Context<Self>,
        theme: Theme,
        settings: Settings,
        live_rates: Arc<Mutex<HashMap<String, f64>>>,
        initial_session: Option<(PathBuf, String)>,
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

        let initial_content = initial_session.as_ref().map(|(_, content)| content.clone());

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
            eval_ctx.set_number_format(this.number_format);
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
                        let fmt = this.number_format;
                        let formatted = format_value_with_precision(
                            &val, &eval_ctx.unit_table, &eval_ctx.currency_table, precision, fmt,
                        );
                        let full_precision = format_value_full_precision(
                            &val, &eval_ctx.unit_table, &eval_ctx.currency_table, fmt,
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
                format_value_with_precision(&total_val, &eval_ctx.unit_table, &eval_ctx.currency_table, precision, this.number_format)
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

            // Auto-save current session
            this.save_current_session(cx);
        })
        .detach();

        // Load initial session content after observer is registered so evaluation fires
        if let Some(content) = initial_content {
            editor.update(cx, |ed, cx| {
                ed.set_content(content, cx);
            });
        }

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
            this.number_format = NumberFormat::from_str(&new_settings.editor.number_format);

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
            let new_num_fmt = this.number_format;
            this.editor.update(cx, |editor, _| {
                editor.font_size = px(new_font_size);
                editor.font_family = SharedString::from(new_font_family);
                editor.number_format = new_num_fmt;
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
            titlebar_should_move: false,
            number_format: NumberFormat::from_str(&settings.editor.number_format),
            session_path: initial_session.map(|(path, _)| path),
            session_list: Vec::new(),
            burger_menu_open: false,
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

    fn toggle_burger_menu(&mut self, cx: &mut Context<Self>) {
        self.burger_menu_open = !self.burger_menu_open;
        if self.burger_menu_open {
            self.session_list = crate::session::list_sessions();
        }
        cx.notify();
    }

    fn switch_session(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.burger_menu_open = false;
        self.save_current_session(cx);
        if let Some(session) = crate::session::load_session(&path) {
            self.session_path = Some(path);
            self.editor.update(cx, |editor, cx| {
                editor.set_content(session.content, cx);
            });
        }
        cx.notify();
    }

    fn new_session(&mut self, cx: &mut Context<Self>) {
        self.burger_menu_open = false;
        self.save_current_session(cx);
        self.session_path = None;
        self.editor.update(cx, |editor, cx| {
            editor.set_content(String::new(), cx);
        });
        cx.notify();
    }

    fn delete_session(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.session_path.as_ref() == Some(&path) {
            return;
        }
        let _ = std::fs::remove_file(&path);
        self.session_list.retain(|(p, _)| p != &path);
        cx.notify();
    }

    fn save_current_session(&mut self, cx: &mut Context<Self>) {
        let content = self.editor.read(cx).content().to_string();

        if content.trim().is_empty() {
            // Empty content: delete the session file if it exists and clear the path.
            // This prevents accumulating empty sessions on disk.
            if let Some(path) = self.session_path.take() {
                let _ = std::fs::remove_file(&path);
            }
            return;
        }

        // Non-empty content: ensure we have a path, creating one lazily if needed.
        let is_new = self.session_path.is_none();
        let path = match &self.session_path {
            Some(path) => path.clone(),
            None => {
                let path = crate::session::new_session_path();
                self.session_path = Some(path.clone());
                path
            }
        };

        let mut session = crate::session::load_session(&path)
            .unwrap_or_else(|| crate::session::Session::new(content.clone()));

        // Only update timestamp when content actually changed. This preserves
        // the original edit time when reopening the app with no new input.
        // New files are always saved so the path is valid on disk.
        if is_new || session.content != content {
            session.content = content;
            session.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            crate::session::save_session(&path, &session);
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
            // Custom "numnum" title bar (follows Zed's pattern:
            // explicit start_window_move on mouse_move, explicit button click handlers)
            .when(self.title_bar == "numnum", |el| {
                let title_color = self.theme.text_muted;
                let bar_bg = self.theme.editor_background;
                let is_macos = cfg!(target_os = "macos");

                let mut bar = div()
                    .id("numnum-titlebar")
                    .w_full()
                    .h(px(32.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .bg(bar_bg)
                    // Zed's drag pattern: on_mouse_down sets flag, on_mouse_move calls start_window_move
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _window, _cx| {
                            this.titlebar_should_move = true;
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _window, _cx| {
                            this.titlebar_should_move = false;
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, _, window, _cx| {
                        if this.titlebar_should_move {
                            this.titlebar_should_move = false;
                            window.start_window_move();
                        }
                    }));

                if is_macos {
                    // macOS: system traffic lights via TitlebarOptions, just pad left
                    bar = bar.child(div().w(px(72.)));
                } else {
                    // Linux/FreeBSD: our own traffic light circles with explicit click handlers
                    let close_color = self.theme.error;
                    let minimize_color = self.theme.syn_variable;
                    let maximize_color = self.theme.result;
                    bar = bar.child(
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
                                    .cursor(CursorStyle::PointingHand)
                                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                        cx.stop_propagation();
                                        window.remove_window();
                                    }),
                            )
                            .child(
                                div()
                                    .id("tb-min")
                                    .size(px(12.))
                                    .rounded_full()
                                    .bg(minimize_color)
                                    .cursor(CursorStyle::PointingHand)
                                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                        cx.stop_propagation();
                                        window.minimize_window();
                                    }),
                            )
                            .child(
                                div()
                                    .id("tb-max")
                                    .size(px(12.))
                                    .rounded_full()
                                    .bg(maximize_color)
                                    .cursor(CursorStyle::PointingHand)
                                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                        cx.stop_propagation();
                                        window.zoom_window();
                                    }),
                            ),
                    );
                }

                let burger_icon_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/burger.svg");

                bar = bar
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_center()
                            .text_size(px(13.))
                            .text_color(title_color)
                            .child("NumNum"),
                    )
                    .child(
                        div()
                            .w(px(68.))
                            .flex()
                            .flex_row()
                            .justify_end()
                            .items_center()
                            .pr(px(12.))
                            .child(
                                div()
                                    .id("burger-btn")
                                    .cursor(CursorStyle::PointingHand)
                                    .child(
                                        svg()
                                            .external_path(burger_icon_path)
                                            .size(px(14.))
                                            .text_color(title_color)
                                    )
                                    .hover(|s| s.opacity(0.7))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.toggle_burger_menu(cx);
                                    }))
                            )
                    );

                el.child(bar)
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
            // Burger menu popup — rendered last so it paints on top
            .when(self.burger_menu_open, |el| {
                let theme = self.theme.clone();
                let current_path = self.session_path.clone();
                let sessions = self.session_list.clone();

                el.child(
                    div()
                        .id("burger-menu")
                        .absolute()
                        .top(titlebar_height)
                        .right(px(8.))
                        .w(px(240.))
                        .max_h(px(320.))
                        .overflow_y_scroll()
                        .bg(theme.editor_background)
                        .border_1()
                        .border_color(theme.divider)
                        .rounded_md()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .p(px(4.))
                                .children(sessions.iter().map(|(path, session)| {
                                    let is_current = current_path.as_ref() == Some(path);
                                    let display_name = crate::session::format_display_name(&session.content);
                                    let timestamp = crate::session::format_timestamp(session.updated_at);
                                    let path_switch = path.clone();
                                    let path_delete = path.clone();
                                    div()
                                        .id(format!("session-item-{}", path_switch.display()))
                                        .group("session_row")
                                        .px(px(8.))
                                        .py(px(5.))
                                        .rounded_sm()
                                        .when(is_current, |el| el.bg(theme.selection))
                                        .when(!is_current, |el| {
                                            el.cursor(CursorStyle::PointingHand)
                                                .hover(|s| s.bg(theme.divider))
                                                .active(|s| s.bg(theme.divider))
                                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                    this.switch_session(path_switch.clone(), cx);
                                                }))
                                        })
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(4.))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .whitespace_nowrap()
                                                        .text_size(px(12.))
                                                        .text_color(theme.text)
                                                        .child(display_name)
                                                )
                                                .child(
                                                    div()
                                                        .flex_shrink_0()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap(px(6.))
                                                        .child(
                                                            div()
                                                                .text_size(px(10.))
                                                                .text_color(theme.text_dimmed)
                                                                .child(timestamp)
                                                        )
                                                        .when(!is_current, |el| {
                                                            el.child(
                                                                div()
                                                                    .id(format!("session-delete-{}", path_delete.display()))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .size(px(16.))
                                                                    .rounded_sm()
                                                                    .text_size(px(11.))
                                                                    .text_color(theme.text_dimmed)
                                                                    .invisible()
                                                                    .group_hover("session_row", |s| s.visible())
                                                                    .hover(|s| s.bg(theme.divider).text_color(theme.text))
                                                                    .child("x")
                                                                    .cursor(CursorStyle::PointingHand)
                                                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                                        cx.stop_propagation();
                                                                        this.delete_session(path_delete.clone(), cx);
                                                                    }))
                                                            )
                                                        })
                                                )
                                        )
                                }))
                                .child(div().h(px(1.)).bg(theme.divider).my(px(4.)))
                                .child(
                                    div()
                                        .id("new-session-btn")
                                        .px(px(8.))
                                        .py(px(6.))
                                        .rounded_sm()
                                        .text_size(px(12.))
                                        .text_color(theme.text)
                                        .cursor(CursorStyle::PointingHand)
                                        .hover(|s| s.bg(theme.divider))
                                        .active(|s| s.bg(theme.divider))
                                        .child("New Session")
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            this.new_session(cx);
                                        }))
                                )
                        )
                        .occlude()
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.burger_menu_open = false;
                            cx.notify();
                        }))
                )
            })
    }
}
