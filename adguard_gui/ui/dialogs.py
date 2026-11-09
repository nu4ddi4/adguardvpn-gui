"""시스템 표시 언어와 관계없이 앱의 언어로 대화상자를 표시합니다."""
from PySide6.QtWidgets import QMessageBox, QWidget

from adguard_gui.i18n import tr


def question(parent: QWidget | None, title: str, text: str) -> bool:
    box = QMessageBox(QMessageBox.Icon.Question, title, text, parent=parent)
    box.setStandardButtons(QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
    box.button(QMessageBox.StandardButton.Yes).setText(tr("Yes"))
    box.button(QMessageBox.StandardButton.No).setText(tr("No"))
    box.setDefaultButton(QMessageBox.StandardButton.No)
    return box.exec() == QMessageBox.StandardButton.Yes


def notice(parent: QWidget | None, title: str, text: str, *, warning: bool = False) -> None:
    icon = QMessageBox.Icon.Warning if warning else QMessageBox.Icon.Information
    box = QMessageBox(icon, title, text, parent=parent)
    box.setStandardButtons(QMessageBox.StandardButton.Ok)
    box.button(QMessageBox.StandardButton.Ok).setText(tr("OK"))
    box.exec()
