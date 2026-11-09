%{!?app_version:%global app_version 0.3.0}
%global debug_package %{nil}
%global app_id io.github.nu4ddi4.AdguardVpnGui

Name:           adguard-gui
Version:        %{app_version}
Release:        1%{?dist}
Summary:        Unofficial AdGuard VPN client for GNOME
License:        MIT
URL:            https://github.com/nu4ddi4/adguardvpn-gui
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.85
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconfig(gtk4) >= 4.12
BuildRequires:  pkgconfig(libadwaita-1) >= 1.4
BuildRequires:  desktop-file-utils

%description
An unofficial Rust and GTK4/libadwaita frontend for the official AdGuard VPN
command-line client. Supports locations, favorites and optional GNOME tray
integration. Install and authenticate adguardvpn-cli separately.

%prep
%autosetup -n %{name}-%{version}

%build
export CARGO_HOME="%{_builddir}/%{name}-cargo"
export CARGO_TARGET_DIR="$PWD/target"
cargo build --release --frozen

%install
%make_install PREFIX=%{_prefix}

%check
# App execution and automated tests are intentionally separate opt-in steps.
desktop-file-validate %{buildroot}%{_datadir}/applications/%{app_id}.desktop

%files
%license LICENSE
%doc README.md VALIDATION.md CHANGELOG.md
%{_bindir}/adguard-gui
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/icons/hicolor/scalable/apps/adguard-gui.svg
%{_datadir}/metainfo/%{app_id}.metainfo.xml

%changelog
* Mon Nov 09 2026 AdGuard VPN GUI contributors - 0.3.0-1
- Replace Qt frontend with Rust and GTK4/libadwaita for Fedora Workstation.
- Vendor dependencies for offline builds; CLI is installed separately.
