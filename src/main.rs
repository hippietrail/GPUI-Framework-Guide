mod app;
mod editor;
mod rates;
mod results_pane;
mod settings_pane;
mod status_bar;
mod theme;

use gpui::{App, Bounds, Focusable, KeyBinding, WindowAppearance, WindowBounds, WindowDecorations, WindowOptions, prelude::*, px, size};
use gpui_platform::application;
use numnum_core::{Settings, ThemeFile};

use crate::app::NumNumApp;
use crate::editor::*;
use crate::settings_pane::EscapeSettings;
use crate::theme::Theme;

fn main() {
    let settings = Settings::load();
    numnum_core::config::ensure_default_themes();

    // Determine which theme to use based on appearance mode
    let theme_name = match settings.appearance.mode.as_str() {
        "dark" => settings.appearance.dark_theme.clone(),
        "light" => settings.appearance.light_theme.clone(),
        _ => {
            // "auto" — will be resolved inside the window callback
            // using window.appearance(); default to dark for pre-window
            settings.appearance.dark_theme.clone()
        }
    };
    let theme_file = ThemeFile::load(&theme_name);
    let theme = Theme::from_theme_file(&theme_file);

    // Start with hardcoded rates (instant), load cached + live in background
    let live_rates = std::sync::Arc::new(std::sync::Mutex::new(
        rates::hardcoded_rates()
    ));

    // Background thread: load cached from SQLite, then fetch live from API
    {
        let rates_ref = live_rates.clone();
        std::thread::spawn(move || {
            let cache = rates::RateCache::new();

            let cached = cache.get_cached_rates();
            if !cached.is_empty() {
                if let Ok(mut rates) = rates_ref.lock() {
                    *rates = cached;
                }
            }

            match cache.fetch_and_store() {
                Some(fresh) => {
                    let count = fresh.len();
                    if let Ok(mut rates) = rates_ref.lock() {
                        *rates = fresh;
                    }
                    eprintln!("[INFO] live exchange rates loaded ({} currencies)", count);
                }
                None => {
                    eprintln!("[INFO] could not fetch live rates, using cached data");
                }
            }
        });
    }

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(settings.window.width), px(settings.window.height)), cx);

        // Bind key bindings for the Editor context
        cx.bind_keys([
            KeyBinding::new("enter", Enter, Some("Editor")),
            KeyBinding::new("backspace", Backspace, Some("Editor")),
            KeyBinding::new("delete", Delete, Some("Editor")),
            KeyBinding::new("left", Left, Some("Editor")),
            KeyBinding::new("right", Right, Some("Editor")),
            KeyBinding::new("up", Up, Some("Editor")),
            KeyBinding::new("down", Down, Some("Editor")),
            KeyBinding::new("shift-left", SelectLeft, Some("Editor")),
            KeyBinding::new("shift-right", SelectRight, Some("Editor")),
            KeyBinding::new("cmd-a", SelectAll, Some("Editor")),
            KeyBinding::new("ctrl-a", SelectAll, Some("Editor")),
            KeyBinding::new("home", Home, Some("Editor")),
            KeyBinding::new("end", End, Some("Editor")),
            KeyBinding::new("cmd-c", Copy, Some("Editor")),
            KeyBinding::new("ctrl-c", Copy, Some("Editor")),
            KeyBinding::new("cmd-x", Cut, Some("Editor")),
            KeyBinding::new("ctrl-x", Cut, Some("Editor")),
            KeyBinding::new("cmd-v", Paste, Some("Editor")),
            KeyBinding::new("ctrl-v", Paste, Some("Editor")),
            KeyBinding::new("cmd-z", Undo, Some("Editor")),
            KeyBinding::new("ctrl-z", Undo, Some("Editor")),
            KeyBinding::new("cmd-shift-z", Redo, Some("Editor")),
            KeyBinding::new("ctrl-shift-z", Redo, Some("Editor")),
            KeyBinding::new("tab", Tab, Some("Editor")),
            KeyBinding::new("escape", Escape, Some("Editor")),
            KeyBinding::new("escape", EscapeSettings, Some("SettingsPane")),
        ]);

        let font_size = settings.editor.font_size;
        let settings_clone = settings.clone();
        let rates_clone = live_rates.clone();

        // Resolve auto mode inside the window callback where we have access to window appearance
        let appearance_mode = settings.appearance.mode.clone();
        let dark_theme_name = settings.appearance.dark_theme.clone();
        let light_theme_name = settings.appearance.light_theme.clone();
        let _pre_window_theme = theme.clone();

        let custom_decorations = matches!(
            settings.window.title_bar.as_str(), "none" | "numnum"
        );
        let _window_handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    focus: true,
                    window_min_size: Some(size(px(480.0), px(360.0))),
                    window_decorations: if custom_decorations {
                        Some(WindowDecorations::Client)
                    } else {
                        None
                    },
                    ..Default::default()
                },
                move |window, cx| {
                    window.set_rem_size(px(font_size));
                    window.set_window_title("NumNum");

                    // For auto mode, start with dark theme (default).
                    // The async XDG portal detection will fire later via
                    // observe_window_appearance and update the theme.
                    let actual_theme = match appearance_mode.as_str() {
                        "light" => Theme::from_theme_file(&ThemeFile::load(&light_theme_name)),
                        "dark" => Theme::from_theme_file(&ThemeFile::load(&dark_theme_name)),
                        _ => Theme::from_theme_file(&ThemeFile::load(&dark_theme_name)),
                    };

                    cx.new(|cx| NumNumApp::new(cx, actual_theme, settings_clone, rates_clone))
                },
            )
            .expect("Failed to open window");

        // Focus the editor on startup + set up auto appearance observer
        let auto_mode = settings.appearance.mode == "auto";
        let dark_name = settings.appearance.dark_theme.clone();
        let light_name = settings.appearance.light_theme.clone();
        _window_handle
            .update(cx, |app, window, cx| {
                window.focus(&app.editor.focus_handle(cx), cx);

                if auto_mode {
                    cx.observe_window_appearance(window, move |this: &mut NumNumApp, window, cx| {
                        let is_dark = matches!(
                            window.appearance(),
                            WindowAppearance::Dark | WindowAppearance::VibrantDark
                        );
                        let name = if is_dark { &dark_name } else { &light_name };
                        let tf = ThemeFile::load(name);
                        let new_theme = Theme::from_theme_file(&tf);
                        this.apply_theme(new_theme, cx);
                    }).detach();
                }

                // Save window size on resize
                cx.observe_window_bounds(window, |_this: &mut NumNumApp, window, _cx| {
                    let size = window.viewport_size();
                    let w: f32 = size.width.into();
                    let h: f32 = size.height.into();
                    if w > 100.0 && h > 100.0 {
                        let mut s = Settings::load();
                        s.window.width = w;
                        s.window.height = h;
                        s.save();
                    }
                }).detach();
            })
            .ok();
    });
}
