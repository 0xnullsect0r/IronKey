//! ironkey-ui — iced 0.14 GUI: three-panel fullscreen application.

pub mod app;
pub mod io_stats;
pub mod modals;
pub mod panels;
pub mod theme;
pub mod widgets;

pub use app::{IronKeyApp, Message};

/// Launch the IronKey GUI.
pub fn run() -> iced::Result {
    use iced::{window, Size};

    // Use free functions to satisfy the `for<'a>` lifetime bounds
    fn boot() -> (IronKeyApp, iced::Task<Message>) {
        IronKeyApp::new()
    }
    fn update(state: &mut IronKeyApp, msg: Message) -> iced::Task<Message> {
        state.update(msg)
    }
    fn view(state: &IronKeyApp) -> iced::Element<'_, Message> {
        state.view()
    }
    fn subscription(state: &IronKeyApp) -> iced::Subscription<Message> {
        state.subscription()
    }
    fn app_theme(_state: &IronKeyApp) -> iced::Theme {
        theme::ironkey_theme()
    }

    iced::application(boot, update, view)
        .title("IronKey v0.1.0")
        .theme(app_theme)
        .subscription(subscription)
        .window(window::Settings {
            size: Size::new(1280.0, 800.0),
            fullscreen: false, // set true when booting on hardware
            decorations: true,
            resizable: true,
            ..Default::default()
        })
        .run()
}
