mod engine;
mod presets;
mod ui;

use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

fn main() {
    // Usage: regex_validator_simulator [PATTERN [INPUT]]
    let mut args = std::env::args().skip(1);
    let (pattern, input) = (args.next(), args.next());
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1380.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("Regex Simulator — Thompson NFA".into()), ..Default::default() }),
                app_id: Some("regex-simulator".into()),
                ..Default::default()
            },
            move |window, cx| cx.new(|cx| ui::app::RegexApp::new(pattern, input, window, cx)),
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
