use gpui::{
    App, Bounds, Context, Render, Window, WindowOptions, WindowBounds,
    div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct NumNum;

impl Render for NumNum {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xf8f8f2))
            .text_size(px(16.))
            .justify_center()
            .items_center()
            .child("numnum")
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: true,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| NumNum),
        )
        .unwrap();
    });
}
