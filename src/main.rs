#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chrome;
mod motion;
mod platform;
#[cfg(all(debug_assertions, feature = "ui-qa"))]
mod qa;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    app::run()
}
