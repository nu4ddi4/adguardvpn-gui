# 패키지 빌드

이번 배포에서는 Fedora용 RPM을 우선 제공합니다.

```bash
sudo dnf install rpm-build make
make rpm
```

`build/rpm/RPMS/noarch/adguard-gui-0.3.1-1.noarch.rpm`에 생성됩니다.
버전을 지정하려면 `make rpm VERSION=0.3.1`을 사용하세요.
소스 압축 파일은 `build/rpm/SOURCES/`에 보관되며 LICENSE와 README를 포함합니다.
빌드에 PySide6나 GUI 실행이 필요하지 않습니다. 설치 시 DNF가 PySide6를 해결합니다.
공식 CLI는 RPM 필수 의존성으로 지정하지 않고 별도 간편 설치에서 처리합니다.

태그 `v0.3.1`을 게시하면 GitHub Actions가 Fedora 43 환경에서 빌드하고,
RPM·설치 스크립트·실행 아이콘·설치 안내·소스 압축 파일·SHA-256 목록을 사전 릴리스에 첨부합니다.
릴리스 태그와 `pyproject.toml`의 버전은 일치해야 하며, 해당 버전의 `docs/releases/` 안내가 필요합니다.
실행·VPN 연결 테스트는 자동으로 수행하지 않습니다.

기존 Debian·Arch·Nix 패키징 파일과 Makefile 진입점은 유지합니다.
없는 빌드 파일을 참조하는 기존 AppImage·Flatpak·Snap 등의 목표는 현재 배포 지원을 의미하지 않습니다.
사용자 설치는 [한국어 설치 안내](../docs/설치-안내.md)를 참고하세요.
