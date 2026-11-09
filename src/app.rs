use crate::{
    cli,
    model::{self, Location, Status},
    preferences::{self, Preferences},
    tray::Tray,
    ui::Ui,
};
use adw::prelude::*;
use gtk::{gio, glib};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    time::Duration,
};

#[derive(Clone, PartialEq)]
enum Request {
    Status,
    Locations,
    Account,
    Config,
    Connect(Option<String>),
    Disconnect,
    Configure(String, String),
}

impl Request {
    fn mutation(&self) -> bool {
        matches!(
            self,
            Self::Connect(_) | Self::Disconnect | Self::Configure(_, _)
        )
    }
}

pub struct Controller {
    application: adw::Application,
    ui: Ui,
    prefs: RefCell<Preferences>,
    status: RefCell<Status>,
    locations: RefCell<Vec<Location>>,
    queue: RefCell<VecDeque<Request>>,
    active: RefCell<Option<Request>>,
    active_generation: Cell<u64>,
    mutation_pending: Cell<bool>,
    config_loaded: Cell<bool>,
    timer: RefCell<Option<glib::SourceId>>,
    tray: RefCell<Option<Rc<Tray>>>,
    hold: RefCell<Option<gio::ApplicationHoldGuard>>,
    autoconnect_checked: Cell<bool>,
    shutting_down: Cell<bool>,
    initialized: Cell<bool>,
    generation: Cell<u64>,
    rows: RefCell<Vec<(Location, adw::ActionRow, gtk::ToggleButton)>>,
}

impl Controller {
    pub fn new(application: &adw::Application) -> Rc<Self> {
        let (prefs, warning) = preferences::load();
        let ui = Ui::new(application, &prefs);
        if let Some(warning) = warning {
            ui.message(&warning);
        }
        Rc::new(Self {
            application: application.clone(),
            ui,
            prefs: RefCell::new(prefs),
            status: RefCell::new(Status::Unknown),
            locations: RefCell::new(Vec::new()),
            queue: RefCell::new(VecDeque::new()),
            active: RefCell::new(None),
            active_generation: Cell::new(0),
            mutation_pending: Cell::new(false),
            config_loaded: Cell::new(false),
            timer: RefCell::new(None),
            tray: RefCell::new(None),
            hold: RefCell::new(None),
            autoconnect_checked: Cell::new(false),
            shutting_down: Cell::new(false),
            initialized: Cell::new(false),
            generation: Cell::new(0),
            rows: RefCell::new(Vec::new()),
        })
    }

    pub fn install(self: &Rc<Self>) {
        for name in [
            "show",
            "connect",
            "disconnect",
            "quit",
            "refresh",
            "locations",
            "about",
        ] {
            let action = gio::SimpleAction::new(name, None);
            let this = self.clone();
            action.connect_activate(move |_, _| this.action(name));
            self.application.add_action(&action);
        }
        self.application
            .set_accels_for_action("app.quit", &["<Control>q"]);
        let this = self.clone();
        self.application.connect_command_line(move |_, command| {
            let args = command.arguments();
            let minimized = args.iter().any(|arg| arg == "--minimized")
                || command
                    .options_dict()
                    .lookup::<bool>("minimized")
                    .ok()
                    .flatten()
                    .unwrap_or(false);
            if args.iter().skip(1).any(|arg| arg != "--minimized") {
                eprintln!("Usage: adguard-gui [--minimized]");
                return 1;
            }
            if !this.initialized.replace(true) {
                this.setup_tray();
                if minimized {
                    // Give the asynchronous watcher registration time to complete.
                    *this.hold.borrow_mut() = Some(this.application.hold());
                    let this = this.clone();
                    glib::timeout_add_local_once(Duration::from_millis(3500), move || {
                        if !this.tray_available() {
                            this.show();
                            this.hold.borrow_mut().take();
                        }
                    });
                } else {
                    this.show();
                }
                for request in [
                    Request::Status,
                    Request::Locations,
                    Request::Account,
                    Request::Config,
                ] {
                    this.enqueue(request);
                }
            } else {
                this.show();
            }
            0
        });
        let this = self.clone();
        self.application.connect_activate(move |_| this.show());
        let weak = Rc::downgrade(self);
        self.application.connect_shutdown(move |_| {
            if let Some(this) = weak.upgrade() {
                this.shutting_down.set(true);
                if let Some(timer) = this.timer.borrow_mut().take() {
                    timer.remove();
                }
                this.tray.borrow_mut().take();
                this.hold.borrow_mut().take();
            }
        });
        let this = self.clone();
        self.ui.window.connect_close_request(move |window| {
            if this.tray_available() {
                window.set_visible(false);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        let this = self.clone();
        self.ui.window.connect_visible_notify(move |_| {
            if this.ui.window.is_visible() {
                this.enqueue(Request::Status);
            }
            this.render_status();
            this.schedule();
        });
        let this = self.clone();
        self.ui
            .connect
            .connect_clicked(move |_| this.enqueue(Request::Connect(None)));
        let this = self.clone();
        self.ui
            .disconnect
            .connect_clicked(move |_| this.enqueue(Request::Disconnect));
        let this = self.clone();
        self.ui
            .search
            .connect_search_changed(move |_| this.filter_locations());
        let this = self.clone();
        self.ui
            .favorites
            .connect_toggled(move |_| this.filter_locations());
        let this = self.clone();
        self.ui
            .preferences_save
            .connect_clicked(move |_| this.save_preferences());
        let this = self.clone();
        self.ui.config_apply.connect_clicked(move |_| {
            let protocol = ["auto", "http2", "quic"][this.ui.protocol.selected().min(2) as usize];
            this.enqueue(Request::Configure(
                protocol.into(),
                this.ui.dns.text().trim().into(),
            ));
        });
        self.render_status();
    }

    fn setup_tray(self: &Rc<Self>) {
        if let Some(connection) = self.application.dbus_connection() {
            let weak = Rc::downgrade(self);
            let availability = Rc::downgrade(self);
            match Tray::new(
                connection,
                Rc::new(move |name| {
                    if let Some(this) = weak.upgrade() {
                        this.action(name);
                    }
                }),
                Rc::new(move |available| {
                    if let Some(this) = availability.upgrade() {
                        this.tray_changed(available);
                    }
                }),
            ) {
                Ok(tray) => *self.tray.borrow_mut() = Some(tray),
                Err(error) => self
                    .ui
                    .message(&format!("Tray registration failed: {error}")),
            }
        }
        self.update_tray_hint(false);
    }

    fn tray_available(&self) -> bool {
        self.tray
            .borrow()
            .as_ref()
            .is_some_and(|tray| tray.available())
    }

    fn update_tray_hint(&self, available: bool) {
        self.ui.tray_hint.set_label(if available {
            "Tray available. Closing this window keeps the app running in the tray."
        } else {
            "No tray host detected. Closing this window exits the GUI and keeps VPN connected. Enable GNOME's AppIndicator extension for a tray icon."
        });
    }

    fn tray_changed(self: &Rc<Self>, available: bool) {
        if self.shutting_down.get() {
            return;
        }
        self.update_tray_hint(available);
        if available {
            if self.hold.borrow().is_none() {
                *self.hold.borrow_mut() = Some(self.application.hold());
            }
        } else {
            self.show();
            self.hold.borrow_mut().take();
        }
    }

    fn action(self: &Rc<Self>, name: &str) {
        match name {
            "show" => self.show(),
            "connect" => self.enqueue(Request::Connect(None)),
            "disconnect" => self.enqueue(Request::Disconnect),
            "locations" => {
                self.ui.message("");
                self.enqueue(Request::Locations);
            }
            "refresh" => {
                self.ui.message("");
                for request in [Request::Status, Request::Account, Request::Config] {
                    self.enqueue(request);
                }
            }
            "quit" => {
                self.shutting_down.set(true);
                self.hold.borrow_mut().take();
                self.application.quit();
            }
            "about" => {
                let about = adw::AboutWindow::builder().transient_for(&self.ui.window).modal(true)
                    .application_name("AdGuard VPN GUI").application_icon("adguard-gui")
                    .version(env!("CARGO_PKG_VERSION")).developer_name("AdGuard VPN GUI contributors")
                    .website("https://github.com/nu4ddi4/adguardvpn-gui")
                    .license_type(gtk::License::MitX11)
                    .comments("Unofficial community frontend. Not affiliated with AdGuard. VPN is provided by the separately installed official CLI.")
                    .build();
                about.present();
            }
            _ => {}
        }
    }

    fn show(self: &Rc<Self>) {
        self.ui.window.present();
    }

    fn enqueue(self: &Rc<Self>, request: Request) {
        if self.shutting_down.get() {
            return;
        }
        if request.mutation() {
            if self.mutation_pending.replace(true) {
                return;
            }
            match &request {
                Request::Connect(_) => *self.status.borrow_mut() = Status::Connecting,
                Request::Disconnect => *self.status.borrow_mut() = Status::Disconnecting,
                _ => {}
            }
            self.queue.borrow_mut().push_front(request);
        } else {
            if (self.active.borrow().as_ref() == Some(&request)
                && self.active_generation.get() == self.generation.get())
                || self.queue.borrow().contains(&request)
            {
                return;
            }
            self.queue.borrow_mut().push_back(request);
        }
        self.render_status();
        self.drain();
    }

    fn drain(self: &Rc<Self>) {
        if self.active.borrow().is_some() || self.shutting_down.get() {
            return;
        }
        let Some(request) = self.queue.borrow_mut().pop_front() else {
            self.schedule();
            return;
        };
        *self.active.borrow_mut() = Some(request.clone());
        let this = self.clone();
        let prefs = self.prefs.borrow().clone();
        let generation = self.generation.get();
        self.active_generation.set(generation);
        glib::spawn_future_local(async move {
            let timeout = if matches!(
                request,
                Request::Connect(_) | Request::Disconnect | Request::Locations
            ) {
                60
            } else {
                prefs.timeout_sec.min(30)
            };
            let args: Vec<String> = match &request {
                Request::Status => vec!["status".into()],
                Request::Locations => vec!["list-locations".into()],
                Request::Account => vec!["license".into()],
                Request::Config => vec!["config".into(), "show".into()],
                Request::Connect(location) => {
                    let mut args = vec!["connect".into(), "-y".into()];
                    if let Some(location) = location {
                        if !location.is_empty() {
                            args.extend(["-l".into(), location.clone()]);
                        }
                    } else {
                        args.push("-f".into());
                    }
                    args
                }
                Request::Disconnect => vec!["disconnect".into()],
                Request::Configure(protocol, _) => {
                    vec!["config".into(), "set-protocol".into(), protocol.clone()]
                }
            };
            let mut result = cli::run(&prefs.binary_path, &args, timeout).await;
            if let Request::Configure(_, dns) = &request {
                if result.is_ok() && !dns.is_empty() {
                    result = cli::run(
                        &prefs.binary_path,
                        &["config".into(), "set-dns".into(), dns.clone()],
                        timeout,
                    )
                    .await;
                }
            }
            if !this.shutting_down.get() && generation == this.generation.get() {
                this.complete(&request, result);
            }
            this.active.borrow_mut().take();
            if request.mutation() {
                this.mutation_pending.set(false);
                this.queue.borrow_mut().retain(|r| *r != Request::Status);
                this.queue.borrow_mut().push_front(Request::Status);
                if matches!(request, Request::Configure(_, _)) {
                    this.queue.borrow_mut().retain(|r| *r != Request::Config);
                    this.queue.borrow_mut().push_front(Request::Config);
                }
            }
            this.render_status();
            this.drain();
        });
    }

    fn complete(self: &Rc<Self>, request: &Request, result: Result<String, String>) {
        match result {
            Err(error) => {
                self.ui.message(&error);
                if *request == Request::Status
                    || matches!(request, Request::Connect(_) | Request::Disconnect)
                {
                    *self.status.borrow_mut() = Status::Unknown;
                }
                if *request == Request::Config {
                    self.config_loaded.set(false);
                    self.ui.config_apply.set_sensitive(false);
                }
                if *request == Request::Account {
                    self.ui.account.set_label("Account unavailable. Run adguardvpn-cli login in a terminal, then refresh.");
                }
                if *request == Request::Locations {
                    self.ui.location_hint.set_label(
                        "Could not refresh locations. Previous results, if any, are retained.",
                    );
                }
            }
            Ok(text) => match request {
                Request::Status => {
                    if self.mutation_pending.get() {
                        return;
                    }
                    let status = model::parse_status(&text);
                    if status == Status::Unknown {
                        self.ui.message(
                            "CLI status output was not recognized. No connection state is assumed.",
                        );
                    }
                    *self.status.borrow_mut() = status.clone();
                    // Trigger only once, after a confirmed disconnected state.
                    if !self.autoconnect_checked.replace(true) && status == Status::Disconnected {
                        let mode = self.prefs.borrow().autoconnect_mode.clone();
                        match mode.as_str() {
                            "last" => self.enqueue(Request::Connect(Some(String::new()))),
                            "fastest" => self.enqueue(Request::Connect(None)),
                            _ => {}
                        }
                    }
                }
                Request::Locations => match model::parse_locations(&text) {
                    Ok(locations) => {
                        *self.locations.borrow_mut() = locations;
                        self.build_locations();
                    }
                    Err(error) => self.ui.message(&error),
                },
                Request::Account => self.ui.account.set_label(&model::clean(&text)),
                Request::Config => {
                    let protocol = model::config_value(&text, "Protocol");
                    let dns = model::config_value(&text, "DNS upstream");
                    if let (Some(protocol), Some(dns)) = (protocol, dns) {
                        self.config_loaded.set(true);
                        let low = protocol.to_lowercase();
                        self.ui.protocol.set_selected(if low.contains("http2") {
                            1
                        } else if low.contains("quic") {
                            2
                        } else {
                            0
                        });
                        self.ui
                            .dns
                            .set_text(if dns.to_lowercase().starts_with("default") {
                                ""
                            } else {
                                &dns
                            });
                        self.ui.protocol.set_sensitive(true);
                        self.ui.dns.set_sensitive(true);
                    } else {
                        self.config_loaded.set(false);
                        self.ui.message("CLI configuration format was not recognized. VPN settings editing is disabled.");
                    }
                }
                _ => self.ui.toast.add_toast(adw::Toast::new(
                    "Command completed; refreshing actual VPN state.",
                )),
            },
        }
    }

    fn schedule(self: &Rc<Self>) {
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.remove();
        }
        if self.shutting_down.get() {
            return;
        }
        let interval = if self.ui.window.is_visible() { 5 } else { 30 };
        let weak = Rc::downgrade(self);
        *self.timer.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_secs(interval),
            move || {
                if let Some(this) = weak.upgrade() {
                    this.timer.borrow_mut().take();
                    this.enqueue(Request::Status);
                    this.schedule();
                }
            },
        ));
    }

    fn render_status(&self) {
        let status = self.status.borrow();
        self.ui.status.set_label(&status.label());
        let busy = self.mutation_pending.get();
        self.ui.connect.set_sensitive(!busy && !status.connected());
        self.ui
            .disconnect
            .set_sensitive(!busy && status.connected());
        self.ui.preferences_save.set_sensitive(!busy);
        self.ui
            .config_apply
            .set_sensitive(!busy && self.config_loaded.get());
        self.ui
            .spinner
            .set_visible(busy && self.ui.window.is_visible());
        self.ui
            .spinner
            .set_spinning(busy && self.ui.window.is_visible());
        self.ui.locations.set_sensitive(!busy);
        if let Some(tray) = self.tray.borrow().as_ref() {
            tray.update(&status);
        }
    }

    fn is_favorite(&self, location: &Location) -> bool {
        let prefs = self.prefs.borrow();
        prefs.favorite_locations.contains(&location.key())
            || prefs.favorite_locations.contains(&location.iso)
    }

    fn build_locations(self: &Rc<Self>) {
        while let Some(child) = self.ui.locations.first_child() {
            self.ui.locations.remove(&child);
        }
        self.rows.borrow_mut().clear();
        for location in self.locations.borrow().iter() {
            let row = adw::ActionRow::builder()
                .title(format!("{} · {}", location.country, location.city))
                .subtitle(
                    location
                        .ping
                        .map(|ping| format!("{} · {ping} ms", location.iso))
                        .unwrap_or_else(|| location.iso.clone()),
                )
                .build();
            let favorite = gtk::ToggleButton::builder()
                .icon_name("starred-symbolic")
                .tooltip_text("Toggle favorite")
                .valign(gtk::Align::Center)
                .active(self.is_favorite(location))
                .build();
            row.add_prefix(&favorite);
            let connect = gtk::Button::builder()
                .label("Connect")
                .valign(gtk::Align::Center)
                .build();
            let this = self.clone();
            let city = location.city.clone();
            connect.connect_clicked(move |_| this.enqueue(Request::Connect(Some(city.clone()))));
            row.add_suffix(&connect);
            let this = self.clone();
            let item = location.clone();
            let rolling_back = Cell::new(false);
            favorite.connect_toggled(move |button| {
                if rolling_back.get() {
                    return;
                }
                let mut prefs = this.prefs.borrow().clone();
                // Expand a legacy country favorite before toggling one city.
                if prefs.favorite_locations.contains(&item.iso) {
                    prefs.favorite_locations.retain(|key| key != &item.iso);
                    for city in this
                        .locations
                        .borrow()
                        .iter()
                        .filter(|loc| loc.iso == item.iso)
                    {
                        if !prefs.favorite_locations.contains(&city.key()) {
                            prefs.favorite_locations.push(city.key());
                        }
                    }
                }
                prefs.favorite_locations.retain(|key| key != &item.key());
                if button.is_active() {
                    prefs.favorite_locations.push(item.key());
                }
                match preferences::save(&prefs) {
                    Ok(()) => *this.prefs.borrow_mut() = prefs,
                    Err(error) => {
                        this.ui.message(&format!("Cannot save favorite: {error}"));
                        let old = this.is_favorite(&item);
                        rolling_back.set(true);
                        button.set_active(old);
                        rolling_back.set(false);
                    }
                }
                this.filter_locations();
            });
            self.ui.locations.append(&row);
            self.rows
                .borrow_mut()
                .push((location.clone(), row, favorite));
        }
        self.filter_locations();
    }

    fn filter_locations(&self) {
        let query = self.ui.search.text().to_lowercase();
        let favorites = self.ui.favorites.is_active();
        let mut count = 0;
        for (location, row, _) in self.rows.borrow().iter() {
            let visible = (!favorites || self.is_favorite(location))
                && format!("{} {} {}", location.iso, location.country, location.city)
                    .to_lowercase()
                    .contains(&query);
            row.set_visible(visible);
            count += usize::from(visible);
        }
        self.ui
            .location_hint
            .set_label(&format!("{count} locations · Refresh to update latency"));
    }

    fn save_preferences(self: &Rc<Self>) {
        let binary = self.ui.binary.text().trim().to_owned();
        if binary.is_empty() {
            self.ui.message("The CLI path cannot be empty.");
            return;
        }
        let old = self.prefs.borrow().clone();
        let mut prefs = old.clone();
        prefs.binary_path = binary;
        prefs.autostart = self.ui.autostart.is_active();
        prefs.autoconnect_mode =
            ["off", "last", "fastest"][self.ui.autoconnect.selected().min(2) as usize].into();
        if let Err(error) = preferences::sync_autostart(prefs.autostart) {
            self.ui
                .message(&format!("Cannot update login startup: {error}"));
            return;
        }
        if let Err(error) = preferences::save(&prefs) {
            let _ = preferences::sync_autostart(old.autostart);
            self.ui
                .message(&format!("Cannot save preferences: {error}"));
            return;
        }
        if old.binary_path != prefs.binary_path {
            self.generation.set(self.generation.get() + 1);
            self.queue.borrow_mut().clear();
            *self.status.borrow_mut() = Status::Unknown;
            self.config_loaded.set(false);
            self.ui.protocol.set_sensitive(false);
            self.ui.dns.set_sensitive(false);
            self.locations.borrow_mut().clear();
            self.build_locations();
        }
        *self.prefs.borrow_mut() = prefs;
        self.ui.message("");
        self.ui
            .toast
            .add_toast(adw::Toast::new("Application settings saved"));
        self.action("refresh");
        self.enqueue(Request::Locations);
    }
}
