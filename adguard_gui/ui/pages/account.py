from __future__ import annotations

from PySide6.QtCore import Qt, QTimer
from PySide6.QtWidgets import (
    QFormLayout,
    QFrame,
    QHBoxLayout,
    QLabel,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from adguard_gui.ui.dialogs import notice, question
from adguard_gui.i18n import tr
from adguard_gui.models.state import AppState
from adguard_gui.models.types import LicenseInfo
from adguard_gui.services.command_runner import CommandRunner
from adguard_gui.services.terminal_login import launch_cli_login
from adguard_gui.theme import (
    ACCENT,
    ACCENT_SOFT,
    BG_CARD,
    BORDER,
    TEXT,
    TEXT_DIM,
    TEXT_MUTED,
    WARNING,
    make_icon,
)
from adguard_gui.ui.widgets.section_header import PageHeader, SectionHeader


def _field(label: str, value: str) -> QWidget:
    box = QVBoxLayout()
    box.setSpacing(2)
    t = QLabel(label)
    t.setStyleSheet(f"color: {TEXT_DIM}; font-size: 11px; letter-spacing: 1px; text-transform: uppercase;")
    v = QLabel(value)
    v.setStyleSheet(f"color: {TEXT}; font-size: 16px; font-weight: 600;")
    v.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
    wrap = QFrame()
    wrap.setLayout(box)
    box.addWidget(t)
    box.addWidget(v)
    return wrap


class AccountPage(QWidget):
    def __init__(self, state: AppState, runner: CommandRunner) -> None:
        super().__init__()
        self.state = state
        self.runner = runner
        self._login_checks_remaining = 0
        self._login_timer = QTimer(self)
        self._login_timer.setInterval(3000)
        self._login_timer.timeout.connect(self._refresh_after_login)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(32, 28, 32, 24)
        outer.setSpacing(18)
        outer.addWidget(PageHeader(tr("Account"), tr("AdGuard VPN authentication and subscription")))
        refresh = QPushButton(tr("Refresh subscription"))
        refresh.clicked.connect(self.runner.license)
        outer.addWidget(refresh)

        self.logged_card = self._build_logged_card()
        self.login_card = self._build_login_card()
        outer.addWidget(self.logged_card)
        outer.addWidget(self.login_card)
        outer.addStretch(1)

        self.state.license_changed.connect(self._render)
        self._render(self.state.license)

    def _build_logged_card(self) -> QFrame:
        card = QFrame()
        card.setObjectName("Card")
        layout = QVBoxLayout(card)
        layout.setContentsMargins(24, 22, 24, 22)
        layout.setSpacing(16)

        header = QHBoxLayout(); header.setSpacing(12)
        icon = QLabel(); icon.setPixmap(make_icon("user", ACCENT, 32).pixmap(32, 32))
        header.addWidget(icon)
        title_box = QVBoxLayout(); title_box.setSpacing(2)
        self.account_email = QLabel("—")
        self.account_email.setStyleSheet(f"color: {TEXT}; font-size: 18px; font-weight: 700;")
        title_box.addWidget(self.account_email)
        self.tier_badge = QLabel("—")
        self.tier_badge.setStyleSheet(
            f"background-color: {ACCENT_SOFT}; color: {ACCENT}; "
            f"padding: 2px 8px; border-radius: 8px; font-size: 11px; font-weight: 700;"
        )
        self.tier_badge.setFixedHeight(20)
        tier_row = QHBoxLayout(); tier_row.setContentsMargins(0, 0, 0, 0); tier_row.setSpacing(6)
        tier_row.addWidget(self.tier_badge); tier_row.addStretch(1)
        title_box.addLayout(tier_row)
        header.addLayout(title_box, 1)

        logout_btn = QPushButton(tr("Sign out"))
        logout_btn.setObjectName("Danger")
        logout_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        logout_btn.clicked.connect(self._logout)
        header.addWidget(logout_btn)
        layout.addLayout(header)

        details_row = QHBoxLayout(); details_row.setSpacing(30)
        self.devices_field = QLabel("—"); self.devices_field.setStyleSheet(f"color: {TEXT}; font-size: 16px; font-weight: 600;")
        self.valid_field = QLabel("—"); self.valid_field.setStyleSheet(f"color: {TEXT}; font-size: 16px; font-weight: 600;")

        for title, widget in [(tr("Devices"), self.devices_field), (tr("Subscription valid until"), self.valid_field)]:
            box = QVBoxLayout(); box.setSpacing(2)
            t = QLabel(title); t.setStyleSheet(f"color: {TEXT_DIM}; font-size: 11px; letter-spacing: 1px; text-transform: uppercase;")
            box.addWidget(t); box.addWidget(widget)
            wrap = QFrame(); wrap.setLayout(box)
            details_row.addWidget(wrap)
        details_row.addStretch(1)
        layout.addLayout(details_row)

        return card

    def _build_login_card(self) -> QFrame:
        card = QFrame(); card.setObjectName("Card")
        layout = QVBoxLayout(card); layout.setContentsMargins(24, 22, 24, 22); layout.setSpacing(12)

        layout.addWidget(SectionHeader(tr("Sign in to AdGuard VPN"), tr("Continue signing in in the official CLI terminal.")))
        hint = QLabel(tr("Complete sign-in in the terminal. Subscription information refreshes automatically for five minutes; use Refresh subscription afterward."))
        hint.setWordWrap(True)
        layout.addWidget(hint)

        btn = QPushButton(tr("Sign in"))
        btn.setObjectName("Primary")
        btn.setCursor(Qt.CursorShape.PointingHandCursor)
        btn.clicked.connect(self._login)
        layout.addWidget(btn)

        return card

    def _render(self, info: LicenseInfo) -> None:
        if info.logged_in:
            self._login_timer.stop()
            self.logged_card.setVisible(True)
            self.login_card.setVisible(False)
            self.account_email.setText(info.email or "—")
            tier = info.tier or "Subscription unknown"
            self.tier_badge.setText(tr(tier))
            if tier == "PREMIUM":
                self.tier_badge.setStyleSheet(
                    f"background-color: {ACCENT_SOFT}; color: {ACCENT}; padding: 2px 8px; border-radius: 8px; font-size: 11px; font-weight: 700;"
                )
            else:
                self.tier_badge.setStyleSheet(
                    f"background-color: rgba(255, 183, 77, 0.18); color: {WARNING}; padding: 2px 8px; border-radius: 8px; font-size: 11px; font-weight: 700;"
                )
            self.devices_field.setText(tr("up to {n}", n=info.device_limit) if info.device_limit else "—")
            self.valid_field.setText(info.valid_until or "—")
        else:
            self.logged_card.setVisible(False)
            self.login_card.setVisible(True)

    def _logout(self) -> None:
        confirmed = question(
            self,
            tr("Confirmation"),
            tr("Sign out of AdGuard VPN? VPN will be disconnected."),
        )
        if confirmed:
            self.runner.logout()
            from PySide6.QtCore import QTimer
            QTimer.singleShot(600, self.runner.license)
            QTimer.singleShot(800, self.runner.status)

    def _login(self) -> None:
        try:
            launch_cli_login(self.state.prefs.binary_path)
        except FileNotFoundError:
            notice(self, tr("Sign in"), tr("VPN CLI was not found. Check its path in Settings."), warning=True)
            return
        except OSError:
            notice(self, tr("Sign in"), tr("Could not open a terminal. Run adguardvpn-cli login in your terminal, then refresh the subscription."), warning=True)
            return
        self._login_checks_remaining = 100
        self._login_timer.start()

    def _refresh_after_login(self) -> None:
        self._login_checks_remaining -= 1
        self.runner.license()
        if self._login_checks_remaining <= 0:
            self._login_timer.stop()
