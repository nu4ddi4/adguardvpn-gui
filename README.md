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

## 빌드

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
