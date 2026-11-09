"""공식 CLI의 버전·업데이트 안내를 분석합니다."""
from __future__ import annotations

import re

from adguard_gui.models.types import UpdateInfo
from adguard_gui.parsers.ansi import strip_ansi

_VERSION = re.compile(r"\bv?(\d+\.\d+\.\d+(?:[-+][\w.-]+)?)\b")
_AVAILABLE = re.compile(
    r"(?:new|latest)\s+version|update\s+(?:is\s+)?available|version\s+available",
    re.IGNORECASE,
)
_UP_TO_DATE = re.compile(
    r"up[ -]to[ -]date|no\s+(?:new\s+)?updates?|already\s+(?:using\s+)?(?:the\s+)?latest",
    re.IGNORECASE,
)


def parse_cli_version(text: str) -> str | None:
    match = _VERSION.search(strip_ansi(text))
    return match.group(1) if match else None


def parse_check_update(text: str, *, current: str | None = None) -> UpdateInfo:
    clean = strip_ansi(text)
    latest = None
    if not _UP_TO_DATE.search(clean):
        for line in clean.splitlines():
            if _AVAILABLE.search(line):
                versions = _VERSION.findall(line)
                if versions:
                    latest = versions[-1]
    return UpdateInfo(
        available=latest is not None and latest != current,
        latest=latest,
        current=current,
        raw=clean,
    )
