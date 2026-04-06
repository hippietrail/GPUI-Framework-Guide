mod app;
mod editor;
mod rates;
mod results_pane;
mod settings_pane;
mod status_bar;
mod theme;

use gpui::{App, Bounds, Focusable, KeyBinding, WindowAppearance, WindowBounds, WindowOptions, prelude::*, px, size};
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
        let bounds = Bounds::centered(None, size(px(900.0), px(640.0)), cx);

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
            KeyBinding::new("escape", EscapeSettings, Some("SettingsPane")),
        ]);

        let font_size = settings.editor.font_size;
        let settings_clone = settings.clone();
        let rates_clone = live_rates.clone();

        // Resolve auto mode inside the window callback where we have access to window appearance
        let appearance_mode = settings.appearance.mode.clone();
        let dark_theme_name = settings.appearance.dark_theme.clone();
        let light_theme_name = settings.appearance.light_theme.clone();
        let pre_window_theme = theme.clone();

        let _window_handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    focus: true,
                    window_min_size: Some(size(px(480.0), px(360.0))),
                    ..Default::default()
                },
                move |window, cx| {
                    window.set_rem_size(px(font_size));

                    // If auto mode, resolve the theme based on window appearance
                    let actual_theme = if appearance_mode == "auto" {
                        let appearance = window.appearance();
                        eprintln!("[INFO] system appearance: {:?}", appearance);
                        let is_dark = matches!(
                            appearance,
                            WindowAppearance::Dark | WindowAppearance::VibrantDark
                        );
                        let name = if is_dark { &dark_theme_name } else { &light_theme_name };
                        Theme::from_theme_file(&ThemeFile::load(name))
                    } else {
                        pre_window_theme
                    };

                    cx.new(|cx| NumNumApp::new(cx, actual_theme, settings_clone, rates_clone))
                },
            )
            .expect("Failed to open window");

        // Focus the editor on startup
        _window_handle
            .update(cx, |app, window, cx| {
                window.focus(&app.editor.focus_handle(cx), cx);
            })
            .ok();
    });
}
