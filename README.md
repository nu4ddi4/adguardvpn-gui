# AdGuard VPN GUI — 한국어판

공식 [`adguardvpn-cli`](https://github.com/AdguardTeam/AdGuardVPNCLI)를 사용하는 **비공식 Linux 데스크톱 앱**입니다.
한국어를 기본으로 제공하며, Fedora용 RPM과 간편 설치를 우선 지원합니다.

## Fedora 설치

[릴리스 다운로드](https://github.com/nu4ddi4/adguardvpn-gui/releases/tag/v0.3.1)에서
`install-fedora.sh`를 내려받은 뒤 해당 폴더에서 실행하세요.

```bash
bash install-fedora.sh
```

또는 터미널에 다음 한 줄을 입력하세요.

```bash
curl --fail --show-error --location --proto '=https' --proto-redir '=https' https://github.com/nu4ddi4/adguardvpn-gui/releases/download/v0.3.1/install-fedora.sh -o install-fedora.sh && bash install-fedora.sh
```

설치 스크립트는 RPM의 SHA-256을 확인하고 DNF로 앱과 의존성을 설치합니다.
공식 VPN CLI가 없으면 AdGuard의 설치 프로그램을 실행합니다.
일반 사용자로 실행하며, 관리자 권한이 필요한 단계에서만 비밀번호를 입력합니다.
설치 후 앱 메뉴의 **AdGuard VPN → 계정**에서 직접 로그인하세요.

- 대상: 일반 Fedora 데스크톱, x86_64·aarch64. Silverblue·Kinoite는 현재 설치 스크립트 대상이 아닙니다.
- Qt 의존성은 Fedora 공식 저장소의 `python3-pyside6 >= 6.7`에서 설치합니다.
- CLI가 이미 설치되어 있으면 다시 설치하지 않습니다.
- RPM 파일은 CPU에 독립적인 `noarch`이며, 공식 CLI는 사용자의 CPU에 맞게 별도 설치됩니다.
- 현재 버전은 사전 배포판입니다. 실제 Fedora 설치·로그인·VPN 연결 테스트는 아직 수행하지 않았습니다.

더 자세한 방법과 삭제 안내는 [설치 안내](docs/설치-안내.md)를 참고하세요.

### RPM만 직접 설치

릴리스에서 `adguard-gui-fedora.noarch.rpm`을 내려받습니다.

```bash
sudo dnf install ./adguard-gui-fedora.noarch.rpm
```

공식 CLI가 없는 경우 간편 설치 스크립트를 사용하세요. RPM의 설치 후 처리에서는 외부 프로그램을 내려받거나 실행하지 않습니다.

## 기능

- VPN 연결·해제와 가장 빠른 서버 선택
- 서버 검색·즐겨찾기·응답 시간 표시와 한국어 지명 검색
- TUN·SOCKS, DNS, 프로토콜, 양자 내성 암호화 설정
- 사이트 예외 목록 및 선택적 VPN 사용
- 로그인·구독 정보, 로그 내보내기, 공식 CLI 업데이트 안내
- 자동 시작·자동 연결, 알림 영역 메뉴
- 한국어 기본 지원, 기존 6개 언어 유지, 실행 중 언어 변경

**연결 끊김 시 자동 재연결**은 CLI의 `--boot` 옵션을 사용합니다.
VPN이 끊긴 동안의 일반 인터넷 트래픽을 방화벽으로 차단하는 기능은 아닙니다.
Fedora GNOME 기본 환경에는 알림 영역 아이콘이 없을 수 있어, 알림 영역이 없는 환경에서는 창을 닫으면 앱이 종료됩니다.

## 한국어 설정

새 설치와 언어 설정이 없는 기존 설정 파일은 한국어를 사용합니다.
이미 저장된 언어는 **설정 → 앱 → 언어 → 한국어**에서 변경할 수 있습니다.
서버 식별자·프로토콜명·외부 CLI의 진단 로그와 공식 설치 프로그램 안내는 원문을 유지합니다.
지명 사전에 없는 새 서버는 원문으로 표시됩니다.

## 소스에서 실행·빌드

Python 3.10 이상과 PySide6 6.7 이상을 사용합니다.

```bash
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
.venv/bin/python main.py
```

Fedora에서 RPM을 빌드하려면 다음을 실행합니다. 빌드 과정에서 GUI를 실행하지 않습니다.

```bash
sudo dnf install rpm-build make
make rpm
```

결과: `build/rpm/RPMS/noarch/adguard-gui-0.3.1-1.noarch.rpm`.
다른 배포판의 기존 패키징 파일은 유지하며, 이번 배포에서는 Fedora RPM을 우선 제공합니다.

## 프로젝트 구조

- `adguard_gui/`: PySide6 앱, CLI 어댑터, 상태·응답 분석
- `adguard_gui/translations_ko.py`: 한국어 문구와 서버 지명
- `packaging/rpm.spec`: Fedora RPM 정의
- `scripts/install-fedora.sh`: Fedora 간편 설치
- `.github/workflows/`: RPM 빌드 및 태그 기반 사전 릴리스 게시

설정은 `~/.config/adguard-gui/config.json`에 저장합니다.
앱은 일반 사용자로 실행하며, VPN 기능은 별도로 설치된 공식 CLI가 처리합니다.

## 저장소와 라이선스

[원본 SpazzRabbit/adguardvpn-gui](https://github.com/SpazzRabbit/adguardvpn-gui)의 포크입니다.
이 포크의 기본 브랜치는 `master`이며, 변경을 별도 작업 브랜치에서 개발하고 병합합니다.
원본 프로젝트 기여는 별도 PR로 진행합니다.

[MIT 라이선스](LICENSE)와 원본 저작권 표기를 유지합니다.
AdGuard Software Ltd.와 제휴하지 않은 커뮤니티 프로젝트입니다.
