# AdGuard VPN GUI — Fedora GNOME

Unofficial desktop frontend for the separately installed official
[AdGuard VPN CLI](https://github.com/AdguardTeam/AdGuardVPNCLI).
Version 0.3.0 replaces the Qt application with **Rust + GTK4/libadwaita**.
Not affiliated with AdGuard. This repository retains the MIT license and history.

## First-version features

- Connect fastest, select a location, disconnect, and read actual CLI status.
- Search locations by country, city or ISO code; save per-city favorites.
- Read account/subscription information; change protocol and DNS upstream.
- Login autostart and optional auto-connect to the last or fastest location.
- Optional tray integration using StatusNotifierItem and DBusMenu over Gio D-Bus.
- Follow GNOME's system appearance; use a single application instance.

The GUI neither installs the VPN CLI nor changes its capabilities or privileges.
Install and authenticate it using the
[official instructions](https://adguard-vpn.com/kb/adguard-vpn-for-linux/installation/).
Run `adguardvpn-cli login` in a terminal, then refresh the app's account information.
The GUI does not handle or store account passwords.

Advanced routing, exclusions, SOCKS settings, CLI updates and GUI authentication
are outside this version's scope. Existing advanced CLI settings are preserved.
The old `--boot` option is not advertised as a kill switch: reconnecting alone
does not guarantee that traffic is blocked while a tunnel is down.

## Fedora Workstation

Requirements: GTK >= 4.12, libadwaita >= 1.4, a session D-Bus, and the official
CLI. The initial UI uses English; the previous Python translations remain in
reference source but are not yet ported.

```sh
sudo dnf install rust cargo gcc make gtk4-devel libadwaita-devel desktop-file-utils rpm-build
make build
sudo make install
adguard-gui
```

This builds the app; it does not execute tests or connect the VPN.

### Tray behavior

GNOME does not provide a tray host by default. Install/enable a compatible
AppIndicator extension if you want a tray icon. The application does not install
or enable Shell extensions automatically.

- With a registered tray host: closing the window hides it and keeps the GUI running.
- Without a tray host: closing the window exits the GUI; the VPN stays connected.
- If a tray host disappears while the app is hidden: the window reappears.
- Launching the app again presents the existing window.
- The tray/application Quit action exits the GUI without disconnecting the VPN.
- `adguard-gui --minimized` starts hidden only when a tray host is available;
  otherwise it opens the window after a short registration grace period.

Use the explicit **Disconnect** action to disconnect the VPN.

### Resource use

One GLib/Gio main loop handles UI, subprocesses and tray traffic; there is no
separate async runtime, worker pool, tray helper process or permanent heartbeat.
CLI commands are serialized. Status requests are deduplicated, with one process
at a time. Status refreshes every 5 seconds with the window visible and every
30 seconds while hidden; opening the window or completing a mutation refreshes
status immediately. No UI animations run while hidden.

Locations, latency, account and configuration are fetched on startup or explicit
refresh, rather than on each status tick. Changing application settings also
refreshes these sources. Location filtering reuses existing rows.

These are implementation choices, not measured performance claims. CPU, memory,
Wayland behavior and actual CLI compatibility still require Fedora acceptance
and performance testing.

## Settings and upgrades

The command remains `adguard-gui`. Settings remain at
`${XDG_CONFIG_HOME:-$HOME/.config}/adguard-gui/config.json`; a legacy file under
`~/.config` is imported when a custom XDG directory is used. Original settings
are backed up as `config.json.pre-gtk` before rewriting. Writes are atomic and
unknown/legacy keys are preserved; malformed source files are not overwritten.

Legacy country favorites are accepted; editing one city's favorite expands the
country entry into city entries. Old tray and kill-switch preference keys are
preserved but no longer control the Rust app.

The old RPM launcher and desktop file are replaced on upgrade. For an old manual
`make install`, remove `/usr/share/applications/adguard-gui.desktop` to avoid a
duplicate menu entry after installing the new application-ID desktop file.
The old Python package may remain on disk but is not loaded by the Rust binary.

Changing DNS uses the CLI's `config set-dns` command. An empty input leaves the
current DNS setting unchanged; use the official CLI to restore its default.

## RPM / SRPM

```sh
make rpm
sudo dnf install ./build/rpm/RPMS/*/adguard-gui-*.rpm
```

`make source` generates a source archive containing `Cargo.lock` and vendored
crates. The RPM compiles with `--frozen`, without downloading dependencies.
It has no `adguardvpn-cli` RPM dependency: an official/manual CLI installation
is detected at runtime. Native GTK/library dependencies are generated by RPM.
Do not use the old multi-distribution recipes with the Rust application.
See [packaging/README.md](packaging/README.md).

## Development and verification

`src/` contains the application, CLI runner, output parsers, settings and tray.
`Cargo.lock` is committed. Files in `adguard_gui/`, `main.py`, `pyproject.toml`,
`requirements.txt` and `scripts/generate_icons.py` are legacy Python reference
source, not the supported application or build entrypoint.

CI build/release workflows are manual (`workflow_dispatch`); releases are drafts.
They do not run the GUI or connect the VPN. Automated tests, desktop acceptance
and performance measurements are opt-in. See [VALIDATION.md](VALIDATION.md) for
the checks to request before release.
