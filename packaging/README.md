# Fedora GTK RPM

`make rpm`으로 CPU별 네이티브 RPM을 생성합니다. 더 이상 noarch Python 패키지가 아닙니다.
GTK 4.20·libadwaita 1.8 이상을 사용하고, Rust 의존성은 Cargo.lock과 함께 소스에 포함합니다.
RPM 빌드 자체는 `cargo build --release --frozen`으로 네트워크 없이 진행합니다.

GitHub Actions는 Fedora 43에서 x86_64·aarch64를 각각 빌드하고,
릴리스에 RPM 두 개·설치 스크립트·설치 아이콘·한국어 안내·소스·체크섬을 첨부합니다.
앱 실행 테스트는 수행하지 않습니다. 예전 Debian·Nix·Arch 파일은 이전 Python 구현의 참고 자료입니다.
