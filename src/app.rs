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
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq)]
enum Request {
    Status,
    Locations,
    Account,
    Config,
    SiteMode,
    Sites(String),
    SiteAdd(String, String),
    SiteRemove(String, String),
    SiteSetMode(String),
    Connect(Option<String>),
    Disconnect,
    Configure(String, String),
}

impl Request {
    fn mutation(&self) -> bool {
        matches!(
            self,
            Self::Connect(_)
                | Self::Disconnect
                | Self::Configure(_, _)
                | Self::SiteAdd(_, _)
                | Self::SiteRemove(_, _)
                | Self::SiteSetMode(_)
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
    account_checked: RefCell<Instant>,
    login_poll_until: RefCell<Option<Instant>>,
    generation: Cell<u64>,
    rows: RefCell<Vec<(Location, adw::ActionRow, gtk::ToggleButton)>>,
    site_current: RefCell<Option<String>>,
    site_loaded: RefCell<Option<String>>,
    update_busy: Cell<bool>,
    update_checked: RefCell<Instant>,
    update_package: RefCell<Option<crate::updater::Package>>,
    installed_version: RefCell<Option<String>>,
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
            account_checked: RefCell::new(Instant::now()),
            login_poll_until: RefCell::new(None),
            generation: Cell::new(0),
            rows: RefCell::new(Vec::new()),
            site_current: RefCell::new(None),
            site_loaded: RefCell::new(None),
            update_busy: Cell::new(false),
            update_checked: RefCell::new(Instant::now()),
            update_package: RefCell::new(None),
            installed_version: RefCell::new(None),
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
                eprintln!("사용법: adguard-gui [--minimized]");
                return 1.into();
            }
            if !this.initialized.replace(true) {
                this.setup_tray();
                let weak = Rc::downgrade(&this);
                glib::timeout_add_local_once(Duration::from_secs(20), move || {
                    if let Some(this) = weak.upgrade() {
                        if this.prefs.borrow().auto_update {
                            this.check_updates();
                        }
                    }
                });
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
                    Request::Account,
                    Request::Status,
                    Request::Config,
                    Request::SiteMode,
                    Request::Locations,
                ] {
                    this.enqueue(request);
                }
            } else {
                this.show();
            }
            0.into()
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
        let this = self.clone();
        self.ui
            .account_refresh
            .connect_clicked(move |_| this.enqueue(Request::Account));
        let this = self.clone();
        self.ui.login.connect_clicked(move |_| {
            let binary = this.prefs.borrow().binary_path.clone();
            match cli::login_in_terminal(&binary) {
                Ok(()) => {
                    *this.login_poll_until.borrow_mut() =
                        Some(Instant::now() + Duration::from_secs(300));
                    this.ui.account.set_label(
                        "터미널에서 로그인을 완료하세요. 5분 동안 구독 정보를 자동 확인합니다.",
                    );
                    this.schedule();
                }
                Err(error) => this.ui.message(&error),
            }
        });
        let this = self.clone();
        self.ui.site_mode.connect_selected_notify(move |_| {
            this.site_loaded.borrow_mut().take();
            this.enqueue(Request::Sites(this.selected_site_mode()));
            this.render_status();
        });
        let this = self.clone();
        self.ui.site_refresh.connect_clicked(move |_| {
            this.enqueue(Request::SiteMode);
            this.enqueue(Request::Sites(this.selected_site_mode()));
        });
        let this = self.clone();
        self.ui.site_apply.connect_clicked(move |_| {
            this.enqueue(Request::SiteSetMode(this.selected_site_mode()))
        });
        let this = self.clone();
        self.ui.site_add.connect_clicked(move |_| {
            match crate::sites::validate(this.ui.site_input.text().as_str()) {
                Ok(value) => this.enqueue(Request::SiteAdd(this.selected_site_mode(), value)),
                Err(error) => this.ui.message(&error),
            }
        });
        let this = self.clone();
        self.ui
            .update_check
            .connect_clicked(move |_| this.check_updates());
        let this = self.clone();
        self.ui
            .update_install
            .connect_clicked(move |_| this.install_update());
        let this = self.clone();
        self.ui.auto_update.connect_active_notify(move |switch| {
            let mut prefs = this.prefs.borrow().clone();
            if prefs.auto_update == switch.is_active() {
                return;
            }
            let previous = prefs.auto_update;
            prefs.auto_update = switch.is_active();
            match preferences::save(&prefs) {
                Ok(()) => {
                    let enabled = prefs.auto_update;
                    *this.prefs.borrow_mut() = prefs;
                    if enabled {
                        this.check_updates();
                    }
                }
                Err(error) => {
                    this.ui
                        .message(&format!("자동 업데이트 설정 저장 실패: {error}"));
                    switch.set_active(previous);
                }
            }
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
                Err(error) => self.ui.message(&format!("알림 영역 등록 실패: {error}")),
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
            "창을 닫으면 알림 영역에서 계속 실행합니다."
        } else {
            "알림 영역이 없습니다. 창을 닫으면 앱만 종료하고 VPN은 유지합니다."
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
                let about = adw::AboutDialog::builder()
                    .application_name("AdGuard VPN GUI").application_icon("adguard-gui")
                    .version(env!("CARGO_PKG_VERSION")).developer_name("AdGuard VPN GUI 기여자")
                    .website("https://github.com/nu4ddi4/adguardvpn-gui")
                    .license_type(gtk::License::MitX11)
                    .comments("AdGuard와 제휴하지 않은 비공식 앱입니다. VPN 연결은 별도로 설치된 공식 CLI가 처리합니다.")
                    .build();
                about.present(Some(&self.ui.window));
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
                Request::SiteMode => vec!["site-exclusions".into(), "mode".into()],
                Request::Sites(mode) => vec![
                    "site-exclusions".into(),
                    "show".into(),
                    "--for-mode".into(),
                    mode.clone(),
                ],
                Request::SiteAdd(mode, value) | Request::SiteRemove(mode, value) => vec![
                    "site-exclusions".into(),
                    if matches!(request, Request::SiteAdd(_, _)) {
                        "add".into()
                    } else {
                        "remove".into()
                    },
                    "--for-mode".into(),
                    mode.clone(),
                    value.clone(),
                ],
                Request::SiteSetMode(mode) => {
                    vec!["site-exclusions".into(), "mode".into(), mode.clone()]
                }
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
            if matches!(
                request,
                Request::SiteAdd(_, _) | Request::SiteRemove(_, _) | Request::SiteSetMode(_)
            ) {
                this.enqueue(Request::SiteMode);
                this.enqueue(Request::Sites(this.selected_site_mode()));
            }
            this.render_status();
            this.drain();
        });
    }

    fn complete(self: &Rc<Self>, request: &Request, result: Result<String, String>) {
        match result {
            Err(error) => {
                if *request != Request::Account {
                    self.ui.message(&error);
                }
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
                    self.ui.account.set_label("구독 조회 실패 — 로그인 여부를 확인하지 못했습니다. 응답 원문을 확인하세요.");
                    if let Ok(account) = model::parse_subscription(&error) {
                        if !account.logged_in {
                            self.ui.account.set_label(&account.display());
                        }
                    }
                    self.ui.account_details.buffer().set_text(&error);
                    *self.account_checked.borrow_mut() = Instant::now();
                }
                if matches!(request, Request::SiteMode | Request::Sites(_)) {
                    self.site_loaded.borrow_mut().take();
                    self.ui
                        .site_status
                        .set_label(&format!("사이트 설정 조회 실패: {error}"));
                }
                if *request == Request::Locations {
                    self.ui
                        .location_hint
                        .set_label("서버 목록을 새로 고치지 못했습니다. 이전 목록을 유지합니다.");
                }
            }
            Ok(text) => match request {
                Request::Status => {
                    if self.mutation_pending.get() {
                        return;
                    }
                    let status = model::parse_status(&text);
                    if status == Status::Unknown {
                        self.ui
                            .message("CLI 응답에서 연결 상태를 확인하지 못했습니다.");
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
                Request::Account => {
                    *self.account_checked.borrow_mut() = Instant::now();
                    self.ui.account_details.buffer().set_text(&format!(
                        "실행 파일: {}\n명령: license\n\n{}",
                        self.prefs.borrow().binary_path,
                        model::clean(&text)
                    ));
                    match model::parse_subscription(&text) {
                        Ok(account) => {
                            self.ui.account.set_label(&account.display());
                            if account.logged_in {
                                self.login_poll_until.borrow_mut().take();
                            }
                        }
                        Err(error) => self.ui.account.set_label(&error),
                    }
                }
                Request::SiteMode => match crate::sites::parse_mode(&text) {
                    Ok(mode) => {
                        let initial = self.site_current.borrow().is_none();
                        *self.site_current.borrow_mut() = Some(mode.clone());
                        if initial {
                            self.ui
                                .site_mode
                                .set_selected(if mode == "selective" { 1 } else { 0 });
                        }
                        self.ui.site_status.set_label(if mode == "selective" {
                            "현재 적용: 목록 사이트만 VPN 사용"
                        } else {
                            "현재 적용: 목록 사이트는 VPN 제외"
                        });
                        self.enqueue(Request::Sites(self.selected_site_mode()));
                    }
                    Err(error) => {
                        self.site_current.borrow_mut().take();
                        self.ui.site_status.set_label(&error);
                    }
                },
                Request::Sites(mode) => {
                    if *mode == self.selected_site_mode() {
                        match crate::sites::parse_entries(&text) {
                            Ok(entries) => {
                                *self.site_loaded.borrow_mut() = Some(mode.clone());
                                self.render_sites(mode, &entries);
                            }
                            Err(error) => {
                                self.site_loaded.borrow_mut().take();
                                self.ui.site_status.set_label(&error);
                            }
                        }
                    }
                }
                Request::SiteAdd(_, _) => {
                    self.ui.site_input.set_text("");
                    self.ui.message("");
                }
                Request::SiteRemove(_, _) | Request::SiteSetMode(_) => {
                    self.ui.message("");
                }
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
                        self.ui
                            .message("CLI 설정 형식을 읽지 못해 설정 편집을 비활성화했습니다.");
                    }
                }
                _ => self.ui.toast.add_toast(adw::Toast::new(
                    "명령을 완료했습니다. 실제 VPN 상태를 확인합니다.",
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
        let login_pending = self
            .login_poll_until
            .borrow()
            .is_some_and(|until| until > Instant::now());
        let interval = if login_pending {
            3
        } else if self.ui.window.is_visible() {
            5
        } else {
            30
        };
        let weak = Rc::downgrade(self);
        *self.timer.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_secs(interval),
            move || {
                if let Some(this) = weak.upgrade() {
                    this.timer.borrow_mut().take();
                    let login_pending = this
                        .login_poll_until
                        .borrow()
                        .is_some_and(|until| until > Instant::now());
                    if login_pending
                        || this.account_checked.borrow().elapsed() >= Duration::from_secs(30)
                    {
                        this.enqueue(Request::Account);
                    }
                    this.enqueue(Request::Status);
                    if this.update_checked.borrow().elapsed() >= Duration::from_secs(6 * 60 * 60)
                        && this.prefs.borrow().auto_update
                    {
                        this.check_updates();
                    }
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
        let site_ready =
            self.site_loaded.borrow().as_deref() == Some(self.selected_site_mode().as_str());
        self.ui.site_add.set_sensitive(!busy && site_ready);
        self.ui.site_list.set_sensitive(!busy && site_ready);
        self.ui
            .site_apply
            .set_sensitive(!busy && self.site_current.borrow().is_some() && site_ready);
        self.ui.site_mode.set_sensitive(!busy);
        self.ui.site_refresh.set_sensitive(!busy);
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
                .title(format!(
                    "{} · {}",
                    crate::places::display(&location.country),
                    crate::places::display(&location.city)
                ))
                .subtitle(
                    location
                        .ping
                        .map(|ping| format!("{} · {ping} ms", location.iso))
                        .unwrap_or_else(|| location.iso.clone()),
                )
                .build();
            let favorite = gtk::ToggleButton::builder()
                .icon_name("starred-symbolic")
                .tooltip_text("즐겨찾기 변경")
                .valign(gtk::Align::Center)
                .active(self.is_favorite(location))
                .build();
            row.add_prefix(&favorite);
            let connect = gtk::Button::builder()
                .label("연결")
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
                        this.ui.message(&format!("즐겨찾기 저장 실패: {error}"));
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
                && format!(
                    "{} {} {} {} {}",
                    location.iso,
                    location.country,
                    location.city,
                    crate::places::display(&location.country),
                    crate::places::display(&location.city)
                )
                .to_lowercase()
                .contains(&query);
            row.set_visible(visible);
            count += usize::from(visible);
        }
        self.ui
            .location_hint
            .set_label(&format!("서버 {count}개 · 새로 고침으로 응답 시간 갱신"));
    }

    fn selected_site_mode(&self) -> String {
        if self.ui.site_mode.selected() == 1 {
            "selective".into()
        } else {
            "general".into()
        }
    }

    fn render_sites(self: &Rc<Self>, mode: &str, entries: &[String]) {
        while let Some(child) = self.ui.site_list.first_child() {
            self.ui.site_list.remove(&child);
        }
        if entries.is_empty() {
            let row = adw::ActionRow::builder()
                .title("목록이 비어 있습니다")
                .build();
            self.ui.site_list.append(&row);
        }
        for entry in entries {
            let row = adw::ActionRow::builder().title(entry).build();
            row.set_use_markup(false);
            let remove = gtk::Button::builder()
                .label("삭제")
                .valign(gtk::Align::Center)
                .build();
            let this = self.clone();
            let mode = mode.to_owned();
            let value = entry.clone();
            remove.connect_clicked(move |_| {
                this.enqueue(Request::SiteRemove(mode.clone(), value.clone()))
            });
            row.add_suffix(&remove);
            self.ui.site_list.append(&row);
        }
    }

    fn check_updates(self: &Rc<Self>) {
        if let Some(version) = self.installed_version.borrow().as_ref() {
            *self.update_checked.borrow_mut() = Instant::now();
            self.ui.update_status.set_label(&format!(
                "{version} 설치 완료. 새 버전을 사용하려면 앱을 다시 실행하세요."
            ));
            return;
        }
        if self.update_busy.replace(true) {
            return;
        }
        *self.update_checked.borrow_mut() = Instant::now();
        self.ui.update_check.set_sensitive(false);
        self.ui.update_install.set_sensitive(false);
        self.ui
            .update_status
            .set_label("새 앱 버전을 확인하고 있습니다…");
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result = crate::updater::prepare(env!("CARGO_PKG_VERSION")).await;
            this.update_busy.set(false);
            this.ui.update_check.set_sensitive(true);
            if this.shutting_down.get() {
                return;
            }
            match result {
                Ok(Some(package)) => {
                    this.ui.update_status.set_label(&format!(
                        "{} 다운로드·체크섬 확인 완료. 설치 시 시스템 인증이 필요할 수 있습니다.",
                        package.version
                    ));
                    *this.update_package.borrow_mut() = Some(package);
                    this.ui.update_install.set_sensitive(true);
                    if this.prefs.borrow().auto_update {
                        this.install_update();
                    }
                }
                Ok(None) => {
                    this.ui
                        .update_status
                        .set_label("현재 앱이 최신 버전입니다.");
                    this.update_package.borrow_mut().take();
                }
                Err(error) => {
                    this.ui
                        .update_status
                        .set_label(&format!("업데이트 확인 실패: {error}"));
                    this.ui
                        .update_install
                        .set_sensitive(this.update_package.borrow().is_some());
                }
            }
        });
    }

    fn install_update(self: &Rc<Self>) {
        if self.update_busy.get() {
            return;
        }
        let Some(package) = self.update_package.borrow().clone() else {
            return;
        };
        self.update_busy.set(true);
        self.ui.update_install.set_sensitive(false);
        self.ui.update_check.set_sensitive(false);
        self.ui
            .update_status
            .set_label("시스템 인증 후 업데이트를 설치합니다. VPN 연결은 유지됩니다.");
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result = crate::updater::install(&package).await;
            this.update_busy.set(false);
            this.ui.update_check.set_sensitive(true);
            if this.shutting_down.get() {
                return;
            }
            match result {
                Ok(()) => {
                    *this.installed_version.borrow_mut() = Some(package.version.clone());
                    this.ui.update_status.set_label(&format!(
                        "{} 설치 완료. 앱을 종료하고 다시 실행하세요.",
                        package.version
                    ));
                    this.update_package.borrow_mut().take();
                }
                Err(error) => {
                    this.ui
                        .update_status
                        .set_label(&format!("업데이트 설치 미완료: {error}"));
                    this.ui.update_install.set_sensitive(true);
                }
            }
        });
    }

    fn save_preferences(self: &Rc<Self>) {
        let binary = self.ui.binary.text().trim().to_owned();
        if binary.is_empty() {
            self.ui.message("CLI 실행 경로를 입력하세요.");
            return;
        }
        let old = self.prefs.borrow().clone();
        let mut prefs = old.clone();
        prefs.binary_path = binary;
        prefs.autostart = self.ui.autostart.is_active();
        prefs.auto_update = self.ui.auto_update.is_active();
        prefs.autoconnect_mode =
            ["off", "last", "fastest"][self.ui.autoconnect.selected().min(2) as usize].into();
        if let Err(error) = preferences::sync_autostart(prefs.autostart) {
            self.ui.message(&format!("자동 시작 설정 실패: {error}"));
            return;
        }
        if let Err(error) = preferences::save(&prefs) {
            let _ = preferences::sync_autostart(old.autostart);
            self.ui.message(&format!("앱 설정 저장 실패: {error}"));
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
            .add_toast(adw::Toast::new("앱 설정을 저장했습니다"));
        self.action("refresh");
        self.enqueue(Request::Locations);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    fn settle(controller: &Rc<Controller>) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            while glib::MainContext::default().pending() {
                glib::MainContext::default().iteration(false);
            }
            if controller.active.borrow().is_none() && controller.queue.borrow().is_empty() {
                break;
            }
            assert!(Instant::now() < deadline, "CLI 요청이 완료되지 않았습니다");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn button(widget: &gtk::Widget, label: &str) -> Option<gtk::Button> {
        if let Some(button) = widget.downcast_ref::<gtk::Button>() {
            if button.label().as_deref() == Some(label) {
                return Some(button.clone());
            }
        }
        let mut child = widget.first_child();
        while let Some(item) = child {
            if let Some(found) = button(&item, label) {
                return Some(found);
            }
            child = item.next_sibling();
        }
        None
    }
    #[test]
    fn subscription_and_site_editor_through_real_gtk_and_cli_transport() {
        let root = std::env::temp_dir().join(format!("adguard-gtk-test-{}", std::process::id()));
        fs::create_dir_all(root.join("config/adguard-gui")).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
        std::env::set_var("XDG_DATA_HOME", root.join("data"));
        std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
        let fixture = root.join("cli.py");
        fs::write(&fixture, include_str!("../tests/fixtures/cli.py")).unwrap();
        fs::set_permissions(&fixture, fs::Permissions::from_mode(0o700)).unwrap();
        let state_path = root.join("state.json");
        std::env::set_var("ADGUARD_GUI_FIXTURE_STATE", &state_path);
        fs::write(
            &state_path,
            r#"{"mode":"general","general":["existing.example"],"selective":[]}"#,
        )
        .unwrap();
        fs::write(
            root.join("config/adguard-gui/config.json"),
            serde_json::json!({"binary_path":fixture,"timeout_sec":5,"auto_update":false})
                .to_string(),
        )
        .unwrap();
        adw::init().unwrap();
        let application = adw::Application::builder()
            .application_id("io.github.nu4ddi4.AdguardVpnGui.Test")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        application.register(None::<&gio::Cancellable>).unwrap();
        let controller = Controller::new(&application);
        controller.install();
        controller.show();
        settle(&controller);
        let set_case = |case: &str| {
            let mut state: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
            state["license_case"] = case.into();
            fs::write(&state_path, state.to_string()).unwrap();
        };
        for case in ["stdout", "stderr"] {
            set_case(case);
            controller.ui.account_refresh.emit_clicked();
            settle(&controller);
            let text = controller.ui.account.text();
            assert!(
                text.contains("프리미엄") && text.contains("10대") && text.contains("2031-08-31"),
                "{case}: {text}"
            );
            assert!(controller
                .ui
                .account_details
                .buffer()
                .text(
                    &controller.ui.account_details.buffer().start_iter(),
                    &controller.ui.account_details.buffer().end_iter(),
                    false
                )
                .contains("license"));
        }
        set_case("error");
        controller.ui.account_refresh.emit_clicked();
        settle(&controller);
        assert!(controller.ui.account.text().contains("조회 실패"));
        set_case("signedout");
        controller.ui.account_refresh.emit_clicked();
        settle(&controller);
        assert!(controller
            .ui
            .account
            .text()
            .contains("로그인되지 않았습니다"));
        set_case("signedout_error");
        controller.ui.account_refresh.emit_clicked();
        settle(&controller);
        assert!(controller
            .ui
            .account
            .text()
            .contains("로그인되지 않았습니다"));
        set_case("unknown");
        controller.ui.account_refresh.emit_clicked();
        settle(&controller);
        assert!(controller
            .ui
            .account
            .text()
            .contains("형식을 인식하지 못했습니다"));
        set_case("stdout");
        controller.ui.account_refresh.emit_clicked();
        settle(&controller);
        assert!(controller.ui.account.text().contains("프리미엄"));
        controller.ui.site_refresh.emit_clicked();
        settle(&controller);
        assert_eq!(controller.site_current.borrow().as_deref(), Some("general"));
        controller.ui.site_mode.set_selected(1);
        settle(&controller);
        controller.ui.site_input.set_text("*.example.com");
        controller.ui.site_add.emit_clicked();
        settle(&controller);
        let state: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(state["selective"][0], "*.example.com");
        assert_eq!(state["general"][0], "existing.example");
        assert_eq!(
            state["mode"], "general",
            "편집 목록 전환만으로 적용 모드를 바꾸면 안 됩니다"
        );
        controller.ui.site_apply.emit_clicked();
        settle(&controller);
        assert_eq!(
            controller.site_current.borrow().as_deref(),
            Some("selective")
        );
        button(controller.ui.site_list.upcast_ref(), "삭제")
            .unwrap()
            .emit_clicked();
        settle(&controller);
        let state: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(state["selective"].as_array().unwrap().len(), 0);
        controller
            .ui
            .site_input
            .set_text("https://bad.example/path");
        controller.ui.site_add.emit_clicked();
        settle(&controller);
        assert!(controller.ui.error.text().contains("URL"));
        set_case("timeout");
        let result = glib::MainContext::default().block_on(cli::run(
            fixture.to_str().unwrap(),
            &["license".into()],
            1,
        ));
        assert!(result.unwrap_err().contains("초과"));
        controller.shutting_down.set(true);
        if let Some(timer) = controller.timer.borrow_mut().take() {
            timer.remove();
        }
        controller.ui.window.close();
        let _ = fs::remove_dir_all(root);
    }
}
