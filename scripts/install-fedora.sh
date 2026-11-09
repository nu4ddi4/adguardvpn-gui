#!/usr/bin/env bash
# Fedora용 간편 설치: RPM 확인 → DNF 설치 → 공식 VPN CLI 설치 안내.
set -Eeuo pipefail

readonly REPOSITORY='nu4ddi4/adguardvpn-gui'
version='0.3.1'
local_rpm=''
install_cli=1

fail() { printf '오류: %s\n' "$*" >&2; exit 1; }
usage() {
    cat <<'HELP'
사용법: bash install-fedora.sh [옵션]
  --version 버전    설치할 릴리스 버전 (기본값: 0.3.1)
  --rpm 파일       내려받은 로컬 RPM 설치 (온라인 체크섬 확인 생략)
  --skip-cli       공식 VPN CLI 설치 생략
  --help           도움말 표시
일반 사용자로 실행하세요. 시스템 패키지 설치 시에만 sudo를 사용합니다.
HELP
}
while (($#)); do
    case "$1" in
        --version) (($# >= 2)) || fail '버전을 입력하세요.'; version="${2#v}"; shift 2 ;;
        --rpm) (($# >= 2)) || fail 'RPM 경로를 입력하세요.'; local_rpm="$2"; shift 2 ;;
        --skip-cli) install_cli=0; shift ;;
        --help|-h) usage; exit 0 ;;
        *) fail "알 수 없는 옵션: $1" ;;
    esac
done
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail '버전 형식은 0.3.1 또는 v0.3.1이어야 합니다.'
[[ "$EUID" -ne 0 ]] || fail 'sudo를 붙이지 말고 일반 사용자로 실행하세요.'
[[ -r /etc/os-release ]] || fail '운영체제 정보를 확인할 수 없습니다.'
# shellcheck source=/dev/null
source /etc/os-release
[[ "${ID:-}" == fedora ]] || fail '이 설치 스크립트는 Fedora용입니다.'
[[ ! -e /run/ostree-booted ]] || fail 'Silverblue·Kinoite 등 불변 시스템은 아직 지원하지 않습니다.'
for tool in curl sha256sum mktemp sudo dnf; do
    command -v "$tool" >/dev/null || fail "필요한 명령이 없습니다: $tool"
done
case "$(uname -m)" in
    x86_64|aarch64) ;;
    *) fail '현재 간편 설치는 x86_64·aarch64를 지원합니다.' ;;
esac

work_dir="$(mktemp -d -t adguard-gui-install.XXXXXXXX)"
trap 'rm -rf -- "$work_dir"' EXIT
trap 'printf "설치가 중단되었습니다. 위 오류를 확인하세요.\n" >&2' ERR
fetch() {
    curl --fail --silent --show-error --location --connect-timeout 15 \
        --max-time 180 --retry 2 --proto '=https' --proto-redir '=https' "$1" -o "$2"
}

if [[ -n "$local_rpm" ]]; then
    [[ -f "$local_rpm" ]] || fail "RPM 파일이 없습니다: $local_rpm"
    rpm_path="$(realpath -- "$local_rpm")"
else
    release_url="https://github.com/$REPOSITORY/releases/download/v$version"
    printf 'AdGuard VPN GUI %s RPM을 내려받고 확인합니다…\n' "$version"
    rpm_path="$work_dir/adguard-gui-fedora.noarch.rpm"
    fetch "$release_url/adguard-gui-fedora.noarch.rpm" "$rpm_path"
    fetch "$release_url/SHA256SUMS" "$work_dir/SHA256SUMS"
    awk '$2 == "adguard-gui-fedora.noarch.rpm" { print }' "$work_dir/SHA256SUMS" > "$work_dir/rpm.sha256"
    [[ "$(wc -l < "$work_dir/rpm.sha256")" -eq 1 ]] || fail 'RPM 체크섬 항목이 없거나 중복되어 있습니다.'
    (cd "$work_dir" && sha256sum --check --status rpm.sha256) || fail 'RPM 체크섬이 일치하지 않습니다.'
fi

printf 'DNF로 앱과 필수 구성 요소를 설치합니다. 관리자 비밀번호가 필요할 수 있습니다.\n'
sudo dnf install -y "$rpm_path" tar gzip util-linux

# 공식 설치 프로그램은 일반 사용자로 실행하고, 권한이 필요한 단계만 sudo를 사용합니다.
export PATH="/usr/local/bin:$PATH"
if ! command -v adguardvpn-cli >/dev/null && [[ ! -x /opt/adguardvpn_cli/adguardvpn-cli ]]; then
    if ((install_cli)); then
        printf 'AdGuard 공식 VPN CLI 설치 프로그램을 내려받습니다…\n'
        fetch 'https://raw.githubusercontent.com/AdguardTeam/AdGuardVPNCLI/master/scripts/release/install.sh' "$work_dir/adguard-official-install.sh"
        printf '공식 설치 프로그램을 실행합니다. 이 단계의 안내는 원문으로 표시됩니다.\n'
        (cd "$work_dir" && timeout 600 sh ./adguard-official-install.sh -v -a y)
        command -v adguardvpn-cli >/dev/null || [[ -x /opt/adguardvpn_cli/adguardvpn-cli ]] || fail '공식 VPN CLI 설치 위치를 확인할 수 없습니다.'
    else
        printf '공식 VPN CLI 설치를 생략했습니다. VPN 사용 전 별도로 설치하세요.\n'
    fi
fi
printf '\n앱 설치가 완료되었습니다. 앱 메뉴에서 AdGuard VPN을 실행하세요.\n'
printf '처음 사용하시면 앱의 [계정]에서 로그인하거나 다음 명령을 실행하세요:\n'
if command -v adguardvpn-cli >/dev/null; then
    printf '  adguardvpn-cli login\n'
else
    printf '  /opt/adguardvpn_cli/adguardvpn-cli login\n'
fi
printf '앱 제거: sudo dnf remove adguard-gui (공식 CLI와 사용자 설정은 유지됩니다.)\n'
