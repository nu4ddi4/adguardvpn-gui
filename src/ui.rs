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
    pub site_mode: adw::ComboRow,
    pub site_apply: gtk::Button,
    pub site_refresh: gtk::Button,
    pub site_input: adw::EntryRow,
    pub site_add: gtk::Button,
    pub site_list: gtk::ListBox,
    pub site_status: gtk::Label,
    pub update_status: gtk::Label,
    pub update_check: gtk::Button,
    pub update_install: gtk::Button,
    pub auto_update: adw::SwitchRow,
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

        let site_page = page();
        let site_group = adw::PreferencesGroup::builder().title("사이트별 VPN 적용")
            .description("일반: 목록에 있는 사이트는 VPN을 우회합니다. 선택: 목록에 있는 사이트에만 VPN을 적용합니다.").build();
        let site_mode = adw::ComboRow::builder()
            .title("적용 방식")
            .model(&gtk::StringList::new(&[
                "목록 사이트 제외",
                "목록 사이트만 적용",
            ]))
            .build();
        let site_apply = gtk::Button::with_label("이 모드로 전환");
        let site_refresh = gtk::Button::with_label("목록 새로 고침");
        site_group.add(&site_mode);
        site_group.add(&site_apply);
        site_group.add(&site_refresh);
        site_page.append(&site_group);
        let site_status = note("현재 모드를 확인하는 중…");
        site_page.append(&site_status);
        let entry_group = adw::PreferencesGroup::builder()
            .title("선택한 모드의 사이트 목록")
            .build();
        let site_input = adw::EntryRow::builder()
            .title("도메인·와일드카드·IP·CIDR")
            .build();
        let site_add = gtk::Button::with_label("목록에 추가");
        entry_group.add(&site_input);
        entry_group.add(&site_add);
        site_page.append(&entry_group);
        site_page.append(&note("루트 도메인과 하위 도메인은 별개입니다. 하위 도메인은 *.example.com처럼 추가하세요. 빈 선택 목록은 사이트를 VPN으로 보내지 않습니다."));
        let site_list = gtk::ListBox::new();
        site_list.set_selection_mode(gtk::SelectionMode::None);
        site_list.add_css_class("boxed-list");
        site_page.append(&site_list);
        site_apply.set_sensitive(false);
        site_add.set_sensitive(false);
        stack.add_titled(
            &gtk::ScrolledWindow::builder()
                .child(&site_page)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("sites"),
            "사이트 예외",
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
            .title("DNS 서버 (비우면 현재 설정 유지)")
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
            "앱별 예외는 지원하지 않습니다. 고급 라우팅과 공식 VPN CLI 업데이트는 CLI에서 관리합니다.",
        ));
        stack.add_titled(
            &gtk::ScrolledWindow::builder()
                .child(&settings)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("settings"),
            "설정",
        );
        let updates = adw::PreferencesGroup::builder()
            .title("앱 자동 업데이트")
            .build();
        let auto_update = adw::SwitchRow::builder()
            .title("새 버전 자동 다운로드·설치")
            .subtitle("사전 배포판을 포함해 6시간마다 확인합니다. 설치 시 관리자 인증이 필요할 수 있습니다.")
            .active(prefs.auto_update)
            .build();
        let update_status = note("업데이트 확인 대기 중");
        let update_check = gtk::Button::with_label("업데이트 확인·다운로드");
        let update_install = gtk::Button::with_label("준비된 업데이트 설치");
        update_install.set_sensitive(false);
        updates.add(&auto_update);
        updates.add(&update_status);
        updates.add(&update_check);
        updates.add(&update_install);
        settings.append(&updates);
        protocol.set_sensitive(false);
        dns.set_sensitive(false);
        config_apply.set_sensitive(false);
        Self {
            window,
            site_mode,
            site_apply,
            site_refresh,
            site_input,
            site_add,
            site_list,
            site_status,
            update_status,
            update_check,
            update_install,
            auto_update,
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
