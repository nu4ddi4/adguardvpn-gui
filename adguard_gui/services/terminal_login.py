"""공식 CLI 로그인을 사용자의 터미널에서 실행합니다."""
from __future__ import annotations

import shutil

from PySide6.QtCore import QProcess


# 서버에 명령을 넘긴 뒤 곧바로 종료하는 터미널도 있어 로그인 완료는 별도 조회합니다.
_TERMINALS = (
    ("xdg-terminal-exec", []),
    ("ptyxis", ["--standalone", "--"]),
    ("gnome-terminal", ["--wait", "--"]),
    ("konsole", ["--separate", "-e"]),
    ("xfce4-terminal", ["--disable-server", "-x"]),
    ("xterm", ["-e"]),
)


def launch_cli_login(binary_path: str) -> None:
    binary = shutil.which(binary_path)
    if not binary:
        raise FileNotFoundError("cli")
    for name, options in _TERMINALS:
        executable = shutil.which(name)
        if executable:
            started, _pid = QProcess.startDetached(executable, [*options, binary, "login"])
            if started:
                return
    raise OSError("terminal")
