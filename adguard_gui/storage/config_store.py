from __future__ import annotations

import json
from dataclasses import asdict
from pathlib import Path

from adguard_gui.models.types import GuiPrefs


class ConfigStore:
    def __init__(self) -> None:
        self.config_path = Path.home() / ".config" / "adguard-gui" / "config.json"

    def load(self) -> GuiPrefs:
        if not self.config_path.exists():
            return GuiPrefs()
        try:
            data = json.loads(self.config_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            return GuiPrefs()
        if not isinstance(data, dict):
            return GuiPrefs()
        defaults = asdict(GuiPrefs())
        merged = {**defaults, **{k: v for k, v in data.items() if k in defaults}}
        try:
            return GuiPrefs(**merged)
        except TypeError:
            return GuiPrefs()

    def save(self, prefs: GuiPrefs) -> None:
        self.config_path.parent.mkdir(parents=True, exist_ok=True)
        self.config_path.write_text(
            json.dumps(asdict(prefs), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
