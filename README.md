# AdGuard VPN GUI — GTK 한국어판

공식 `adguardvpn-cli`를 사용하는 비공식 Fedora GNOME 앱입니다.
0.4.0부터 Rust·GTK 4·libadwaita로 실행하며 Qt·PySide6를 사용하지 않습니다.

## 설치와 업데이트

[v0.4.0 릴리스](https://github.com/nu4ddi4/adguardvpn-gui/releases/tag/v0.4.0)의
`install-fedora.sh`를 내려받아 일반 사용자로 실행하세요.

```bash
bash install-fedora.sh
```

기존 앱은 종료한 뒤 새 앱을 실행하세요. 설치 스크립트는 CPU에 맞는 RPM을 선택하고
체크섬 확인 후 DNF로 설치합니다. 공식 CLI가 없으면 AdGuard 설치 프로그램을 실행합니다.
Fedora 43 이상 일반 데스크톱의 x86_64·aarch64용이며 Silverblue·Kinoite는 설치 스크립트 대상이 아닙니다.
RPM을 직접 설치하려면 `sudo dnf install ./adguard-gui-fedora.x86_64.rpm`을 사용하세요.

## 로그인과 구독 정보

**공식 CLI 로그인**을 누르면 터미널에서 로그인을 진행합니다.
앱은 비밀번호를 입력받지 않습니다. 로그인 후 5분 동안 자동 조회하고,
이후에도 30초 간격 또는 **구독 정보 새로 고침**으로 확인합니다.

계정·구독 화면에서 다음을 구분합니다.

- 조회 성공: 계정, 구독 등급, 최대 기기 수, 만료일
- 미로그인: 공식 CLI가 미로그인 상태라고 명시한 경우
- 조회 실패: 실행 실패·시간 초과·인식하지 못한 응답

실패 시 **구독 조회 응답 원문**을 펼쳐 실제 실행 경로와 CLI 응답을 확인하세요.
터미널에서 확인한 CLI와 앱의 설정 경로가 같은지도 확인해야 합니다.
앱과 CLI를 일반 사용자로 실행하며, 다른 사용자 또는 sudo로 로그인한 계정은 별개입니다.
지원 문의 시 응답 원문의 이메일 등 개인정보를 가리고 전달해주세요.

## 화면과 기능

- 한국어 화면·서버 지명 검색·즐겨찾기
- VPN 연결·해제, 가장 빠른 서버, 프로토콜·DNS 설정
- 공식 CLI 터미널 로그인, 구독 조회와 응답 진단
- 자동 시작·자동 연결, 선택적 알림 영역 연동
- 설정 화면에서 실제 GTK·libadwaita 런타임 버전 확인

GTK 전환판에서는 고급 라우팅·상세 SOCKS 설정·사이트 예외 편집·CLI 업데이트를 공식 CLI로 관리합니다.
기존 VPN 설정을 자동 초기화하지 않으며, Python UI 소스는 이전 구현 참고용으로 보존합니다.
이전 GUI 설정 파일의 기타 키도 보존하고, 설정 저장 시 `.json.pre-gtk` 백업을 만듭니다.
알림 영역이 없는 GNOME에서는 창을 닫으면 앱만 종료하며 VPN은 유지됩니다.

## 광고 차단과 선택 앱 VPN 연동 (0.4.1)

광고 차단 자동 모드와 VPN TUN 모드는 함께 사용하지 않습니다. 사전에 VPN을
SOCKS / 127.0.0.1:1080으로, 광고 차단의 outbound proxy를 같은 주소로 구성해야 합니다.
광고 차단의 apps 목록은 `vpn-apps.yaml` 대상만 전달하고 다른 앱은
`skip_outbound_proxy: true`로 제외해야 합니다. Tailscale·로컬 주소는 VPN 예외로 둡니다.

GUI 설정 `adguard_integration: true`와 `adguard_binary_path`로 연동을 활성화합니다.
연결 버튼은 SOCKS 모드·실제 VPN 상태·로컬 포트·자동 광고 차단을 확인한 다음
대상 앱 전달을 켭니다. 명시적인 연결 해제만 전달을 끄고 VPN을 종료합니다.
VPN 장애나 상태 조회 실패 시에는 전달을 임의 해제하지 않습니다.
상태가 불명이어도 연결 해제로 일반 연결로 전환할 수 있습니다.

연결 화면의 **VPN 대상 목록 열기**에서 실행 파일 이름을 관리합니다.
기본 대상은 Chrome이며 모든 Chrome 프로필·웹앱에 적용될 수 있습니다.
게임의 UDP·특수 포트·Wine 공통 프로세스는 별도 검토가 필요합니다.
DNS는 시스템·Tailscale 설정을 유지하며 모든 DNS가 VPN을 경유한다고 보장하지 않습니다.
창을 닫아도 VPN과 전달 설정은 유지합니다.

앱 실행·실제 VPN 접속 테스트는 수행하지 않았습니다.

## 빌드

광고 차단 엔진의 로그인 자동 시작은 GUI 트레이 자동 시작과 별개입니다.
`scripts/adguard-engine-login.py`를 사용자 libexec 폴더에 설치하고
`~/.local/bin/adguard-engine-login`으로 연결합니다.
`packaging/adguard-engine-login.desktop`를 사용자 autostart 폴더에 설치하면
그래픽 로그인 때 기존 GUI와 같은 세션 환경에서 시작합니다.
이미 실행 중이면 재시작하지 않습니다. 시작 명령은 로그인당 한 번만 시도하고
상태 확인은 최대 세 번으로 제한합니다. daemon의 stdout 상속 때문에
시작 완료를 시간 초과로 오인하지 않도록 파이프 EOF를 기다리지 않습니다.
GUI에서 사용자가 광고 차단을 끄면 그 세션에서 다시 강제로 켜지 않습니다.

Fedora 43의 GTK 4.20 이상·libadwaita 1.8 이상과 Rust 1.92 이상이 필요합니다.
Rust 바인딩은 gtk4 0.11·libadwaita 0.9를 사용하고 Cargo.lock으로 버전을 고정합니다.

```bash
sudo dnf install cargo rust gcc gtk4-devel libadwaita-devel pkgconf-pkg-config make rpm-build desktop-file-utils
cargo build --release --locked
make rpm
```

RPM은 의존성 소스를 포함한 압축 파일을 만든 뒤 오프라인으로 컴파일합니다.
앱 실행·로그인·실제 VPN 연결 테스트는 별도 요청 전까지 수행하지 않습니다.

## 저장소와 라이선스

[원본 프로젝트](https://github.com/SpazzRabbit/adguardvpn-gui)의 포크입니다.
기존 Rust 작업 브랜치를 바탕으로 한국어·로그인·구독 진단·Fedora 배포 경로를 통합했습니다.
[MIT 라이선스](LICENSE)와 원본 저작권을 유지합니다. AdGuard와 제휴하지 않은 커뮤니티 앱입니다.
