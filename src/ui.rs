use adw::prelude::*;
use gtk::gio;

pub struct Ui {
    pub window: adw::ApplicationWindow,
    pub toast: adw::ToastOverlay,
    pub status: gtk::Label,
    pub spinner: gtk::Spinner,
    pub connect: gtk::Button,
    pub disconnect: gtk::Button,
    pub search: gtk::SearchEntry,
    pub favorites: gtk::ToggleButton,
    pub locations: gtk::ListBox,
    pub location_hint: gtk::Label,
    pub account: gtk::Label,
    pub protocol: adw::ComboRow,
    pub dns: adw::EntryRow,
    pub binary: adw::EntryRow,
    pub autostart: adw::SwitchRow,
    pub autoconnect: adw::ComboRow,
    pub config_apply: gtk::Button,
    pub preferences_save: gtk::Button,
    pub tray_hint: gtk::Label,
    pub error: gtk::Label,
}

fn page() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build()
}

fn note(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.0)
        .selectable(true)
        .css_classes(["dim-label"])
        .build()
}

impl Ui {
    pub fn new(app: &adw::Application, prefs: &crate::preferences::Preferences) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("AdGuard VPN")
            .icon_name("adguard-gui")
            .default_width(660)
            .default_height(720)
            .width_request(360)
            .height_request(420)
            .build();
        let toolbar = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        let menu = gio::Menu::new();
        menu.append(Some("Refresh account and settings"), Some("app.refresh"));
        menu.append(Some("About AdGuard VPN GUI"), Some("app.about"));
        menu.append(Some("Quit (keep VPN connected)"), Some("app.quit"));
        header.pack_end(
            &gtk::MenuButton::builder()
                .icon_name("open-menu-symbolic")
                .menu_model(&menu)
                .build(),
        );
        toolbar.add_top_bar(&header);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let error = gtk::Label::builder()
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .margin_start(16)
            .margin_end(16)
            .margin_top(8)
            .margin_bottom(8)
            .css_classes(["error"])
            .visible(false)
            .build();
        content.append(&error);
        let stack = gtk::Stack::builder().vexpand(true).build();
        let switcher = gtk::StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk::Align::Center)
            .margin_top(8)
            .build();
        content.append(&switcher);
        content.append(&stack);
        toolbar.set_content(Some(&content));
        let toast = adw::ToastOverlay::new();
        toast.set_child(Some(&toolbar));
        window.set_content(Some(&toast));

        let home = page();
        let icon = gtk::Image::from_icon_name("network-vpn-symbolic");
        icon.set_pixel_size(64);
        home.append(&icon);
        let status = gtk::Label::builder()
            .label("Reading VPN status…")
            .wrap(true)
            .css_classes(["title-1"])
            .build();
        home.append(&status);
        let spinner = gtk::Spinner::builder()
            .halign(gtk::Align::Center)
            .visible(false)
            .build();
        home.append(&spinner);
        let buttons = gtk::Box::builder()
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();
        let connect = gtk::Button::with_label("Connect fastest");
        connect.add_css_class("suggested-action");
        let disconnect = gtk::Button::with_label("Disconnect");
        disconnect.add_css_class("destructive-action");
        buttons.append(&connect);
        buttons.append(&disconnect);
        home.append(&buttons);
        let tray_hint = note("Checking tray support…");
        home.append(&tray_hint);
        home.append(&note(
            "Closing this app never disconnects the VPN. Use Disconnect to end the connection.",
        ));
        let account_group = adw::PreferencesGroup::builder().title("Account").build();
        let account = note("Reading account information…");
        account_group.add(&account);
        let help = gtk::LinkButton::with_label(
            "https://adguard-vpn.com/kb/adguard-vpn-for-linux/installation/",
            "Official CLI installation and login instructions",
        );
        help.set_halign(gtk::Align::Start);
        account_group.add(&help);
        home.append(&account_group);
        home.append(&note("Install and authenticate adguardvpn-cli separately. This unofficial app does not collect your password."));
        stack.add_titled(
            &gtk::ScrolledWindow::builder()
                .child(&home)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("home"),
            "Connection",
        );

        let servers = page();
        let search = gtk::SearchEntry::builder()
            .placeholder_text("Search country, city or ISO code")
            .build();
        servers.append(&search);
        let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let favorites = gtk::ToggleButton::with_label("Favorites only");
        filters.append(&favorites);
        let refresh = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh locations")
            .action_name("app.locations")
            .build();
        filters.append(&refresh);
        servers.append(&filters);
        let location_hint = note("Loading locations…");
        servers.append(&location_hint);
        let locations = gtk::ListBox::new();
        locations.set_selection_mode(gtk::SelectionMode::None);
        locations.add_css_class("boxed-list");
        servers.append(&locations);
        stack.add_titled(
            &gtk::ScrolledWindow::builder()
                .child(&servers)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("locations"),
            "Locations",
        );

        let settings = page();
        let group = adw::PreferencesGroup::builder()
            .title("VPN settings")
            .description("Read from the official CLI. Existing advanced settings remain unchanged.")
            .build();
        let protocol = adw::ComboRow::builder()
            .title("Protocol")
            .model(&gtk::StringList::new(&["Automatic", "HTTP/2", "QUIC"]))
            .build();
        let dns = adw::EntryRow::builder()
            .title("DNS upstream (empty = keep current)")
            .build();
        group.add(&protocol);
        group.add(&dns);
        let config_apply = gtk::Button::with_label("Apply VPN settings");
        config_apply.set_halign(gtk::Align::End);
        group.add(&config_apply);
        settings.append(&group);
        let group = adw::PreferencesGroup::builder()
            .title("Application")
            .build();
        let binary = adw::EntryRow::builder()
            .title("Path to adguardvpn-cli")
            .text(&prefs.binary_path)
            .build();
        let autostart = adw::SwitchRow::builder()
            .title("Launch at login")
            .active(prefs.autostart)
            .build();
        let autoconnect = adw::ComboRow::builder()
            .title("Connect when this app starts")
            .model(&gtk::StringList::new(&["Off", "Last location", "Fastest"]))
            .build();
        autoconnect.set_selected(match prefs.autoconnect_mode.as_str() {
            "last" => 1,
            "fastest" => 2,
            _ => 0,
        });
        group.add(&binary);
        group.add(&autostart);
        group.add(&autoconnect);
        let preferences_save = gtk::Button::with_label("Save application settings");
        preferences_save.set_halign(gtk::Align::End);
        group.add(&preferences_save);
        settings.append(&group);
        settings.append(&note("GNOME's AppIndicator extension is optional. With a tray host, closing the window keeps this app in the tray; without one, the GUI exits and the VPN keeps running. Hidden status refresh: 30 seconds; visible: 5 seconds."));
        settings.append(&note("Exclusions, advanced routing and CLI updates are managed with the official command line in this version. Automatic reconnect is not a traffic-blocking kill switch."));
        stack.add_titled(
            &gtk::ScrolledWindow::builder()
                .child(&settings)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("settings"),
            "Settings",
        );
        protocol.set_sensitive(false);
        dns.set_sensitive(false);
        config_apply.set_sensitive(false);
        Self {
            window,
            toast,
            status,
            spinner,
            connect,
            disconnect,
            search,
            favorites,
            locations,
            location_hint,
            account,
            protocol,
            dns,
            binary,
            autostart,
            autoconnect,
            config_apply,
            preferences_save,
            tray_hint,
            error,
        }
    }

    pub fn message(&self, message: &str) {
        self.error.set_label(message);
        self.error.set_visible(!message.is_empty());
    }
}
