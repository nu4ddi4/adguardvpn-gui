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
    pub account_details: gtk::TextView,
    pub login: gtk::Button,
    pub account_refresh: gtk::Button,
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
        menu.append(Some("계정·설정 새로 고침"), Some("app.refresh"));
        menu.append(Some("AdGuard VPN GUI 정보"), Some("app.about"));
        menu.append(Some("종료 (VPN 연결 유지)"), Some("app.quit"));
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
            .label("VPN 상태 확인 중…")
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
        let connect = gtk::Button::with_label("가장 빠른 서버에 연결");
        connect.add_css_class("suggested-action");
        let disconnect = gtk::Button::with_label("연결 해제");
        disconnect.add_css_class("destructive-action");
        buttons.append(&connect);
        buttons.append(&disconnect);
        home.append(&buttons);
        let tray_hint = note("알림 영역 지원 확인 중…");
        home.append(&tray_hint);
        home.append(&note(
            "앱을 종료해도 VPN은 연결을 유지합니다. 연결 해제 버튼으로 종료하세요.",
        ));
        if prefs.adguard_integration {
            home.append(&note("VPN 대상 목록에 있는 앱만 광고 차단을 거쳐 VPN을 사용합니다. 초기 대상은 Chrome입니다. Tailscale의 기기망·DNS 설정은 유지합니다."));
            let targets = gtk::Button::with_label("VPN 대상 목록 열기");
            let target_window = window.clone();
            let target_toast = toast.clone();
            targets.connect_clicked(move |_| {
                let file = gio::File::for_path(crate::integration::targets_path());
                let messages = target_toast.clone();
                gtk::FileLauncher::new(Some(&file)).launch(
                    Some(&target_window),
                    gio::Cancellable::NONE,
                    move |result| {
                        if let Err(error) = result {
                            messages.add_toast(adw::Toast::new(&format!("대상 목록을 열지 못했습니다: {error}")));
                        }
                    },
                );
            });
            home.append(&targets);
        }
        let account_group = adw::PreferencesGroup::builder().title("계정·구독").build();
        let account = note("구독 정보 확인 중…");
        account_group.add(&account);
        let login = gtk::Button::with_label("공식 CLI 로그인");
        let account_refresh = gtk::Button::with_label("구독 정보 새로 고침");
        let account_buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        account_buttons.append(&login);
        account_buttons.append(&account_refresh);
        account_group.add(&account_buttons);
        let diagnostics = adw::ExpanderRow::builder()
            .title("구독 조회 응답 원문")
            .build();
        let account_details = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(8)
            .right_margin(8)
            .build();
        diagnostics.add_row(&account_details);
        account_group.add(&diagnostics);
        let help = gtk::LinkButton::with_label(
            "https://adguard-vpn.com/kb/adguard-vpn-for-linux/installation/",
            "공식 CLI 설치·로그인 안내",
        );
        help.set_halign(gtk::Align::Start);
        account_group.add(&help);
        home.append(&account_group);
        home.append(&note(
            "공식 CLI 로그인은 터미널에서 진행합니다. 앱에서 비밀번호를 입력받지 않습니다.",
        ));
        stack.add_titled(
            &gtk::ScrolledWindow::builder()
                .child(&home)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("home"),
            "연결",
        );

        let servers = page();
        let search = gtk::SearchEntry::builder()
            .placeholder_text("국가·도시·국가 코드로 검색")
            .build();
        servers.append(&search);
        let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let favorites = gtk::ToggleButton::with_label("즐겨찾기만 표시");
        filters.append(&favorites);
        let refresh = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("서버 목록 새로 고침")
            .action_name("app.locations")
            .build();
        filters.append(&refresh);
        servers.append(&filters);
        let location_hint = note("서버 목록을 불러오는 중…");
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
            "서버 위치",
        );

        let settings = page();
        let group = adw::PreferencesGroup::builder()
            .title("VPN 설정")
            .description("공식 CLI에서 불러온 연결 설정입니다.")
            .build();
        let protocol = adw::ComboRow::builder()
            .title("프로토콜")
            .model(&gtk::StringList::new(&["자동", "HTTP/2", "QUIC"]))
            .build();
        let dns = adw::EntryRow::builder()
            .title(if prefs.adguard_integration {
                "DNS는 Tailscale·시스템 설정을 유지합니다"
            } else {
                "DNS 서버 (비우면 현재 설정 유지)"
            })
            .build();
        group.add(&protocol);
        group.add(&dns);
        let config_apply = gtk::Button::with_label("VPN 설정 적용");
        config_apply.set_halign(gtk::Align::End);
        group.add(&config_apply);
        settings.append(&group);
        let group = adw::PreferencesGroup::builder().title("앱 설정").build();
        let binary = adw::EntryRow::builder()
            .title("adguardvpn-cli 실행 파일 경로")
            .text(&prefs.binary_path)
            .build();
        let autostart = adw::SwitchRow::builder()
            .title("로그인 시 자동 시작")
            .active(prefs.autostart)
            .build();
        let autoconnect = adw::ComboRow::builder()
            .title("앱 시작 시 자동 연결")
            .model(&gtk::StringList::new(&[
                "사용 안 함",
                "마지막 서버",
                "가장 빠른 서버",
            ]))
            .build();
        autoconnect.set_selected(match prefs.autoconnect_mode.as_str() {
            "last" => 1,
            "fastest" => 2,
            _ => 0,
        });
        group.add(&binary);
        group.add(&autostart);
        group.add(&autoconnect);
        let preferences_save = gtk::Button::with_label("앱 설정 저장");
        preferences_save.set_halign(gtk::Align::End);
        group.add(&preferences_save);
        settings.append(&group);
        settings.append(&note(&format!(
            "앱 {} · GTK {}.{}.{} · libadwaita {}.{}.{}",
            env!("CARGO_PKG_VERSION"),
            gtk::major_version(),
            gtk::minor_version(),
            gtk::micro_version(),
            adw::major_version(),
            adw::minor_version(),
            adw::micro_version()
        )));
        settings.append(&note("알림 영역이 지원되면 창을 닫아도 앱을 유지합니다. 알림 영역이 없으면 앱만 종료하고 VPN은 유지합니다."));
        settings.append(&note(
            "고급 라우팅과 CLI 업데이트는 공식 CLI에서 관리합니다.",
        ));
        stack.add_titled(
            &gtk::ScrolledWindow::builder()
                .child(&settings)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("settings"),
            "설정",
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
            account_details,
            login,
            account_refresh,
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
