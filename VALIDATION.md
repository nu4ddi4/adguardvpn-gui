# Opt-in validation checklist

No automated tests, GUI launches, VPN connections or performance measurements
are performed without a separate request. Compilation and static source/package
inspection are distinct from running tests.

## Parser tests

When requested: `cargo test --locked`.
Included cases cover multi-word cities, ANSI output, unknown status, missing
latency, malformed location lists and configuration values containing colons.
Inputs are synthetic examples derived from legacy parsers, not captured output
from an authenticated current CLI. Collect actual output on Fedora before
claiming version compatibility; omit account addresses and credentials.

## Fedora desktop and CLI acceptance

- Workstation GNOME/Wayland: install, open, reopen, system light/dark appearance,
  narrow window, keyboard access and screen reader labels.
- CLI missing, wrong path, unauthenticated, permission denied, timeout, and changed
  output format: actionable errors, no assumed disconnected/connected state.
- Connect fastest, select a city, disconnect; repeated clicks do not overlap CLI
  mutations. Failed mutations are followed by an actual status query.
- Account and basic configuration read; protocol/DNS writes; partial config write
  failure refreshes actual settings. Empty DNS input preserves the setting.
- Tray extension enabled/disabled, host restart/disappearance, startup hidden,
  tray menu actions, window reopening and quit while VPN remains connected.
- External CLI connect/disconnect: reflected within 5 seconds when visible or
  30 seconds when hidden, plus the duration of an in-flight CLI command.
- Autostart off/on, last/fastest auto-connect only after confirmed disconnected
  status; already connected, unknown or failed status never auto-connects.
- Existing settings, country favorites, custom XDG_CONFIG_HOME, malformed JSON,
  read-only settings directory, backup creation and save failure handling.
- RPM install, upgrade from 0.2.0, uninstall and reinstall; desktop menu identity,
  icon, shared-library dependencies, no Python dependency, offline source rebuild.

## Performance

On the same Fedora machine/CLI version, record idle CPU, RSS/PSS, wakeups and
CLI process counts for at least five minutes with the window visible and hidden.
Compare equal connection states; record GTK renderer and display scaling.
Verify that account/location/config commands do not repeat at each status tick,
only one CLI child runs at a time, and hidden UI spinners stop.
Treat the 5/30-second intervals as scheduling policy, not a latency guarantee
while another CLI command is running. Do not claim numerical savings without
recorded measurements.
