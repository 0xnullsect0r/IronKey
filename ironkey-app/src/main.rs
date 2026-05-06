//! IronKey — entry point.
//!
//! Boots directly into the fullscreen iced application.

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    log::info!("IronKey v{} starting", env!("CARGO_PKG_VERSION"));

    if let Err(e) = ironkey_ui::run() {
        log::error!("Application error: {:?}", e);
        std::process::exit(1);
    }
}
