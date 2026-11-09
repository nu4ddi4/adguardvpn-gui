mod app;
mod cli;
mod model;
mod preferences;
mod tray;
mod ui;

use adw::prelude::*;
use gtk::{gio, glib};

const APP_ID: &str = "io.github.nu4ddi4.AdguardVpnGui";

fn main() -> glib::ExitCode {
    let application = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    application.add_main_option(
        "minimized",
        0u8.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Start in the tray when a host is available",
        None,
    );
    application.connect_startup(|application| {
        let controller = app::Controller::new(application);
        controller.install();
    });
    application.run()
}
