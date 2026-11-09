# Fedora Workstation packaging

The supported 0.3.x package is a native, architecture-specific Fedora RPM.
Old Debian, Flatpak, Snap and other recipes are retained as historical reference;
they still describe the Python/Qt application and are not supported for Rust.

## Build

Install `rust cargo gcc make gtk4-devel libadwaita-devel desktop-file-utils rpm-build`.
From the repository root:

```sh
make build   # Rust release compilation, no GUI execution
make source  # locked crate vendoring and source archive
make rpm     # offline RPM compilation, desktop entry validation, RPM and SRPM
```

Artifacts are under `build/rpm/RPMS/` and `build/rpm/SRPMS/`.
`Source0` contains the Rust sources, lockfile, Cargo vendor configuration,
vendored crates (including upstream license files), assets and package metadata.
The archive root matches `%autosetup -n adguard-gui-<version>`.
`cargo build --release --frozen` in the RPM never downloads crates.

RPM runtime library requirements are automatic. The official VPN CLI is installed
and authenticated separately, so a nonexistent distribution package does not
block DNF installation. The GUI reports CLI installation, authentication and
permission errors without silently installing or elevating anything.

`BuildArch: noarch`, Python/PySide6 dependencies, Python launchers, Qt icon
generation, and old icon/desktop cache scriptlets are no longer used.
Fedora's standard file triggers update desktop and icon caches.

Local RPM builds use bundled crates. This is not a claim of Fedora official
repository compliance: official submission would also require Fedora Rust
packaging policy and dependency/license review. No repository publication is
performed by local build commands.
