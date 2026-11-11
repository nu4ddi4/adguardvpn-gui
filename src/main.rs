mod app;
mod cli;
mod integration;
mod model;
mod places;
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
        "알림 영역이 지원되면 최소화 상태로 시작",
        None,
    );
    application.connect_startup(|application| {
        let controller = app::Controller::new(application);
        controller.install();
    });
    application.run()
}
