use iced::{Font, Settings, Size, application, window};
use swap_rescue::gui::SwapRescueApp;

fn main() {
    let settings = Settings {
        default_font: Font::with_name("Golos Text"),
        ..Default::default()
    };

    let _ = application(SwapRescueApp::title, SwapRescueApp::update, SwapRescueApp::view)
        .settings(settings)
        .window(window::Settings {
            size: Size::new(820.0, 980.0),
            min_size: Some(Size::new(640.0, 720.0)),
            ..Default::default()
        })
        .font(include_bytes!("../assets/GolosText-Regular.ttf"))
        .run_with(SwapRescueApp::new);
}
