%{!?app_version:%global app_version 0.3.0}

Name:           adguard-gui
Version:        %{app_version}
Release:        1%{?dist}
Summary:        한국어 AdGuard VPN 데스크톱 앱
License:        MIT
URL:            https://github.com/nu4ddi4/adguardvpn-gui
Source0:        %{name}-%{version}.tar.gz
BuildArch:      noarch

Requires:       python3 >= 3.10
Requires:       python3-pyside6 >= 6.7
Requires:       hicolor-icon-theme

%description
공식 adguardvpn-cli를 사용하는 비공식 Qt 데스크톱 앱입니다.
한국어 기본 화면, 서버 선택, 예외 목록과 자동 시작을 지원합니다.
공식 VPN CLI는 간편 설치 스크립트에서 별도로 설치합니다.

%prep
%setup -q

%build
# 순수 Python 앱과 SVG 아이콘이므로 GUI 실행이나 아이콘 변환이 필요 없습니다.

%install
mkdir -p %{buildroot}%{_datadir}/adguard-gui
cp -r adguard_gui %{buildroot}%{_datadir}/adguard-gui/
install -Dm755 packaging/adguard-gui %{buildroot}%{_bindir}/adguard-gui
install -Dm644 packaging/adguard-gui.desktop %{buildroot}%{_datadir}/applications/adguard-gui.desktop
install -Dm644 assets/icons/adguard-gui.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/adguard-gui.svg

%files
%license LICENSE
%doc README.md
%{_datadir}/adguard-gui/
%{_bindir}/adguard-gui
%{_datadir}/applications/adguard-gui.desktop
%{_datadir}/icons/hicolor/scalable/apps/adguard-gui.svg

%post
if ! command -v adguardvpn-cli >/dev/null 2>&1 && [ ! -x /opt/adguardvpn_cli/adguardvpn-cli ]; then
    echo '공식 VPN CLI가 필요합니다. 릴리스의 install-fedora.sh를 일반 사용자로 실행하세요.'
fi

%changelog
* Mon Nov 09 2026 AdGuard VPN GUI contributors - 0.3.0-1
- 한국어 기본 지원과 Fedora RPM 소스 빌드·설치 경로 정리.
