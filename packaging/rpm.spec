%{!?app_version:%global app_version 0.4.1}
%global debug_package %{nil}
%global app_id io.github.nu4ddi4.AdguardVpnGui
Name:           adguard-gui
Version:        %{app_version}
Release:        1%{?dist}
Summary:        Fedora GNOME용 한국어 AdGuard VPN 앱
License:        MIT
URL:            https://github.com/nu4ddi4/adguardvpn-gui
Source0:        %{name}-%{version}.tar.gz
BuildRequires:  python3
BuildRequires:  cargo
BuildRequires:  rust >= 1.92
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconfig(gtk4) >= 4.20
BuildRequires:  pkgconfig(libadwaita-1) >= 1.8
BuildRequires:  desktop-file-utils
Requires:       gtk4 >= 4.20
Requires:       libadwaita >= 1.8
Requires:       hicolor-icon-theme
%description
공식 adguardvpn-cli를 사용하는 Rust·GTK 4·libadwaita 데스크톱 앱입니다.
한국어 화면, 서버 선택, 즐겨찾기, 구독 정보 조회와 터미널 로그인을 제공합니다.
공식 CLI는 간편 설치 스크립트로 별도 설치합니다.
%prep
%autosetup -n %{name}-%{version}
%build
export CARGO_HOME="%{_builddir}/%{name}-cargo"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
cargo build --release --frozen
%install
%make_install PREFIX=%{_prefix} CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
%check
# 앱 실행과 로그인·VPN 연결 테스트는 수행하지 않습니다.
desktop-file-validate %{buildroot}%{_datadir}/applications/%{app_id}.desktop
%files
%license LICENSE licenses
%doc README.md CHANGELOG.md
%{_bindir}/adguard-gui
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/icons/hicolor/scalable/apps/adguard-gui.svg
%{_datadir}/metainfo/%{app_id}.metainfo.xml
%changelog
* Mon Nov 09 2026 AdGuard VPN GUI contributors - 0.4.0-1
- GTK 4·libadwaita 전환 및 구독 조회·진단 화면 추가.
