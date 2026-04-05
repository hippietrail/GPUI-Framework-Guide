mod app;
mod editor;
mod results_pane;
mod status_bar;
mod theme;

use gpui::{App, Bounds, KeyBinding, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_platform::application;
use numnum_core::Settings;

use crate::app::NumNumApp;
use crate::editor::*;
use crate::theme::Theme;

fn main() {
    let settings = Settings::load();
    let theme = Theme::from_settings(&settings);

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
        ]);

        let theme_clone = theme.clone();
        let font_family = settings.editor.font_family.clone();
        let font_size = settings.editor.font_size;
        let _window_handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    focus: true,
                    ..Default::default()
                },
                move |window, cx| {
                    window.set_rem_size(px(font_size));
                    cx.new(|cx| NumNumApp::new(cx, theme_clone, font_family, font_size))
                },
            )
            .expect("Failed to open window");
    });
}
