//! StatusNotifierItem and the small DBusMenu subset needed by our static menu.
//! All traffic shares Gio's main context; there is no extra runtime or polling.
use glib::variant::ToVariant;
use gtk::{gio, glib};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";
const MENU: &str = "com.canonical.dbusmenu";
const XML: &str = r#"<node>
<interface name="org.kde.StatusNotifierItem">
 <method name="Activate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
 <method name="SecondaryActivate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
 <method name="ContextMenu"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
 <method name="Scroll"><arg type="i" direction="in"/><arg type="s" direction="in"/></method>
 <property name="Category" type="s" access="read"/>
 <property name="Id" type="s" access="read"/>
 <property name="Title" type="s" access="read"/>
 <property name="Status" type="s" access="read"/>
 <property name="WindowId" type="u" access="read"/>
 <property name="IconName" type="s" access="read"/>
 <property name="IconPixmap" type="a(iiay)" access="read"/>
 <property name="OverlayIconName" type="s" access="read"/>
 <property name="OverlayIconPixmap" type="a(iiay)" access="read"/>
 <property name="AttentionIconName" type="s" access="read"/>
 <property name="AttentionIconPixmap" type="a(iiay)" access="read"/>
 <property name="AttentionMovieName" type="s" access="read"/>
 <property name="ToolTip" type="(sa(iiay)ss)" access="read"/>
 <property name="ItemIsMenu" type="b" access="read"/>
 <property name="Menu" type="o" access="read"/>
 <signal name="NewIcon"/>
 <signal name="NewToolTip"/>
 <signal name="NewStatus"><arg type="s"/></signal>
</interface>
<interface name="com.canonical.dbusmenu">
 <property name="Version" type="u" access="read"/>
 <property name="TextDirection" type="s" access="read"/>
 <property name="Status" type="s" access="read"/>
 <property name="IconThemePath" type="as" access="read"/>
 <method name="GetLayout"><arg type="i" direction="in"/><arg type="i" direction="in"/><arg type="as" direction="in"/><arg type="u" direction="out"/><arg type="(ia{sv}av)" direction="out"/></method>
 <method name="GetGroupProperties"><arg type="ai" direction="in"/><arg type="as" direction="in"/><arg type="a(ia{sv})" direction="out"/></method>
 <method name="GetProperty"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="out"/></method>
 <method name="Event"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="in"/><arg type="u" direction="in"/></method>
 <method name="EventGroup"><arg type="a(isvu)" direction="in"/><arg type="ai" direction="out"/></method>
 <method name="AboutToShow"><arg type="i" direction="in"/><arg type="b" direction="out"/></method>
 <method name="AboutToShowGroup"><arg type="ai" direction="in"/><arg type="ai" direction="out"/><arg type="ai" direction="out"/></method>
 <signal name="LayoutUpdated"><arg type="u"/><arg type="i"/></signal>
</interface></node>"#;

type Action = Rc<dyn Fn(&str)>;
type Availability = Rc<dyn Fn(bool)>;

pub struct Tray {
    connection: gio::DBusConnection,
    available: Cell<bool>,
    label: RefCell<String>,
    icon: RefCell<String>,
    on_action: Action,
    on_available: Availability,
    registrations: RefCell<Vec<gio::RegistrationId>>,
    watcher: RefCell<Option<Box<dyn FnOnce()>>>,
    signals: RefCell<Vec<gio::SignalSubscriptionId>>,
    epoch: Cell<u64>,
}

impl Tray {
    pub fn new(
        connection: gio::DBusConnection,
        on_action: Action,
        on_available: Availability,
    ) -> Result<Rc<Self>, glib::Error> {
        let tray = Rc::new(Self {
            connection,
            available: Cell::new(false),
            label: RefCell::new("Status unavailable".into()),
            icon: RefCell::new("dialog-warning-symbolic".into()),
            on_action,
            on_available,
            registrations: RefCell::new(Vec::new()),
            watcher: RefCell::new(None),
            signals: RefCell::new(Vec::new()),
            epoch: Cell::new(0),
        });
        let info = gio::DBusNodeInfo::for_xml(XML)?;
        for (path, interface) in [("/StatusNotifierItem", ITEM), ("/MenuBar", MENU)] {
            let weak = Rc::downgrade(&tray);
            let properties = Rc::downgrade(&tray);
            let registration = tray
                .connection
                .register_object(path, &info.lookup_interface(interface).unwrap())
                .method_call(move |_, _, _, interface, method, parameters, invocation| {
                    if let Some(tray) = weak.upgrade() {
                        tray.method(interface.unwrap_or(""), method, parameters, invocation);
                    }
                })
                .property(move |_, _, _, interface, name| {
                    properties
                        .upgrade()
                        .map(|tray| tray.property(interface, name))
                        .unwrap_or_else(|| "".to_variant())
                })
                .build()?;
            tray.registrations.borrow_mut().push(registration);
        }
        let weak = Rc::downgrade(&tray);
        let gone = Rc::downgrade(&tray);
        let id = gio::bus_watch_name_on_connection(
            &tray.connection,
            WATCHER,
            gio::BusNameWatcherFlags::NONE,
            move |_, _, _| {
                if let Some(tray) = weak.upgrade() {
                    tray.register();
                }
            },
            move |_, _| {
                if let Some(tray) = gone.upgrade() {
                    tray.epoch.set(tray.epoch.get() + 1);
                    tray.set_available(false);
                }
            },
        );
        *tray.watcher.borrow_mut() = Some(Box::new(move || gio::bus_unwatch_name(id)));
        for (interface, member) in [
            (WATCHER, "StatusNotifierHostRegistered"),
            (WATCHER, "StatusNotifierHostUnregistered"),
            ("org.freedesktop.DBus.Properties", "PropertiesChanged"),
        ] {
            let weak = Rc::downgrade(&tray);
            let id = tray.connection.signal_subscribe(
                Some(WATCHER),
                Some(interface),
                Some(member),
                Some("/StatusNotifierWatcher"),
                None,
                gio::DBusSignalFlags::NONE,
                move |_, _, _, _, _, _| {
                    if let Some(tray) = weak.upgrade() {
                        tray.query_host();
                    }
                },
            );
            tray.signals.borrow_mut().push(id);
        }
        Ok(tray)
    }

    fn register(self: &Rc<Self>) {
        self.epoch.set(self.epoch.get() + 1);
        let epoch = self.epoch.get();
        let tray = self.clone();
        glib::spawn_future_local(async move {
            let result = tray
                .connection
                .call_future(
                    Some(WATCHER),
                    "/StatusNotifierWatcher",
                    WATCHER,
                    "RegisterStatusNotifierItem",
                    Some(&("/StatusNotifierItem",).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    3000,
                )
                .await;
            if epoch == tray.epoch.get() {
                if result.is_ok() {
                    tray.query_host();
                } else {
                    tray.set_available(false);
                }
            }
        });
    }

    fn query_host(self: &Rc<Self>) {
        let tray = self.clone();
        let epoch = self.epoch.get();
        glib::spawn_future_local(async move {
            let result = tray
                .connection
                .call_future(
                    Some(WATCHER),
                    "/StatusNotifierWatcher",
                    "org.freedesktop.DBus.Properties",
                    "Get",
                    Some(&(WATCHER, "IsStatusNotifierHostRegistered").to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    3000,
                )
                .await;
            let available = result
                .ok()
                .and_then(|v| v.child_value(0).as_variant())
                .and_then(|v| v.get::<bool>())
                .unwrap_or(false);
            if epoch == tray.epoch.get() {
                tray.set_available(available);
            }
        });
    }

    fn set_available(&self, available: bool) {
        if self.available.replace(available) != available {
            (self.on_available)(available);
        }
    }

    pub fn available(&self) -> bool {
        self.available.get()
    }

    pub fn update(&self, status: &crate::model::Status) {
        let label = status.label();
        if *self.label.borrow() == label {
            return;
        }
        *self.label.borrow_mut() = label;
        *self.icon.borrow_mut() = match status {
            crate::model::Status::Connected(_) => "network-vpn-symbolic",
            crate::model::Status::Disconnected => "network-offline-symbolic",
            _ => "dialog-warning-symbolic",
        }
        .into();
        for signal in ["NewIcon", "NewToolTip"] {
            let _ = self
                .connection
                .emit_signal(None, "/StatusNotifierItem", ITEM, signal, None);
        }
        let mut changed = HashMap::new();
        for property in ["IconName", "ToolTip"] {
            changed.insert(property, self.property(ITEM, property));
        }
        let _ = self.connection.emit_signal(
            None,
            "/StatusNotifierItem",
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
            Some(&(ITEM, changed, Vec::<String>::new()).to_variant()),
        );
    }

    fn property(&self, interface: &str, name: &str) -> glib::Variant {
        let pixmaps: Vec<(i32, i32, Vec<u8>)> = Vec::new();
        if interface == MENU {
            return match name {
                "Version" => 3u32.to_variant(),
                "TextDirection" => "ltr".to_variant(),
                "Status" => "normal".to_variant(),
                "IconThemePath" => Vec::<String>::new().to_variant(),
                _ => "".to_variant(),
            };
        }
        match name {
            "Category" => "Communications".to_variant(),
            "Id" => "adguard-gui".to_variant(),
            "Title" => "AdGuard VPN".to_variant(),
            "Status" => "Active".to_variant(),
            "WindowId" => 0u32.to_variant(),
            "IconName" => self.icon.borrow().as_str().to_variant(),
            "IconPixmap" | "OverlayIconPixmap" | "AttentionIconPixmap" => pixmaps.to_variant(),
            "AttentionIconName" => "dialog-warning-symbolic".to_variant(),
            "ToolTip" => (
                self.icon.borrow().as_str(),
                pixmaps,
                "AdGuard VPN",
                self.label.borrow().as_str(),
            )
                .to_variant(),
            "ItemIsMenu" => false.to_variant(),
            "Menu" => glib::variant::ObjectPath::try_from("/MenuBar")
                .unwrap()
                .to_variant(),
            _ => "".to_variant(),
        }
    }

    fn menu_properties(id: i32, names: &[String]) -> HashMap<String, glib::Variant> {
        let mut properties = HashMap::new();
        if id == 0 {
            properties.insert("children-display".into(), "submenu".to_variant());
        } else if let Some(label) = [
            "",
            "Open window",
            "Connect (fastest)",
            "Disconnect",
            "Quit (keep VPN connected)",
        ]
        .get(id as usize)
        {
            properties.insert("label".into(), label.to_variant());
            properties.insert("enabled".into(), true.to_variant());
            properties.insert("visible".into(), true.to_variant());
        }
        if !names.is_empty() {
            properties.retain(|name, _| names.contains(name));
        }
        properties
    }

    fn layout(id: i32, depth: i32, names: &[String]) -> glib::Variant {
        let children = if id == 0 && depth != 0 {
            (1..=4)
                .map(|id| Self::layout(id, 0, names))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        (id, Self::menu_properties(id, names), children).to_variant()
    }

    fn event(&self, id: i32, event: &str) {
        if event != "clicked" {
            return;
        }
        if let Some(action) = ["", "show", "connect", "disconnect", "quit"].get(id as usize) {
            (self.on_action)(action);
        }
    }

    fn method(
        &self,
        interface: &str,
        method: &str,
        p: glib::Variant,
        call: gio::DBusMethodInvocation,
    ) {
        let reply = if interface == ITEM {
            if method == "Activate" || method == "SecondaryActivate" {
                (self.on_action)("show");
            }
            ().to_variant()
        } else {
            match method {
                "GetLayout" => {
                    let (id, depth, names) = p.get::<(i32, i32, Vec<String>)>().unwrap();
                    if !(0..=4).contains(&id) {
                        call.return_dbus_error(
                            "com.canonical.dbusmenu.Error.InvalidMenuID",
                            "Unknown menu item",
                        );
                        return;
                    }
                    glib::Variant::tuple_from_iter([
                        1u32.to_variant(),
                        Self::layout(id, depth, &names),
                    ])
                }
                "GetGroupProperties" => {
                    let (mut ids, names) = p.get::<(Vec<i32>, Vec<String>)>().unwrap();
                    if ids.is_empty() {
                        ids = (0..=4).collect();
                    }
                    let rows: Vec<_> = ids
                        .into_iter()
                        .filter(|id| (0..=4).contains(id))
                        .map(|id| (id, Self::menu_properties(id, &names)))
                        .collect();
                    (rows,).to_variant()
                }
                "GetProperty" => {
                    let (id, name) = p.get::<(i32, String)>().unwrap();
                    if let Some(value) = Self::menu_properties(id, &[]).remove(&name) {
                        (value,).to_variant()
                    } else {
                        call.return_dbus_error(
                            "com.canonical.dbusmenu.Error.InvalidProperty",
                            "Unknown property",
                        );
                        return;
                    }
                }
                "Event" => {
                    let (id, event, _, _) = p.get::<(i32, String, glib::Variant, u32)>().unwrap();
                    self.event(id, &event);
                    ().to_variant()
                }
                "EventGroup" => {
                    let (events,) = p
                        .get::<(Vec<(i32, String, glib::Variant, u32)>,)>()
                        .unwrap();
                    for (id, event, _, _) in events {
                        self.event(id, &event);
                    }
                    (Vec::<i32>::new(),).to_variant()
                }
                "AboutToShow" => (false,).to_variant(),
                "AboutToShowGroup" => (Vec::<i32>::new(), Vec::<i32>::new()).to_variant(),
                _ => {
                    call.return_dbus_error(
                        "org.freedesktop.DBus.Error.UnknownMethod",
                        "Unknown method",
                    );
                    return;
                }
            }
        };
        call.return_value(Some(&reply));
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        if let Some(watcher) = self.watcher.get_mut().take() {
            watcher();
        }
        for signal in self.signals.get_mut().drain(..) {
            self.connection.signal_unsubscribe(signal);
        }
        for registration in self.registrations.get_mut().drain(..) {
            let _ = self.connection.unregister_object(registration);
        }
    }
}
