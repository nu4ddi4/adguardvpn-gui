use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::{collections::BTreeMap, env, fs, io, os::unix::fs::OpenOptionsExt, path::PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub binary_path: String,
    pub timeout_sec: u32,
    pub favorite_locations: Vec<String>,
    pub autostart: bool,
    pub autoconnect_mode: String,
    // Keep legacy and future preferences when saving with this version.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            binary_path: default_binary(),
            timeout_sec: 30,
            favorite_locations: Vec::new(),
            autostart: false,
            autoconnect_mode: "off".into(),
            extra: BTreeMap::new(),
        }
    }
}

fn config_home() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .filter(|v| PathBuf::from(v).is_absolute())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"))
}

pub fn path() -> PathBuf {
    config_home().join("adguard-gui/config.json")
}

pub fn load() -> (Preferences, Option<String>) {
    let file = path();
    // Older releases ignored XDG_CONFIG_HOME. Read their file if necessary.
    let legacy = PathBuf::from(env::var_os("HOME").unwrap_or_default())
        .join(".config/adguard-gui/config.json");
    let source = if file.exists() { file } else { legacy };
    match fs::read(&source) {
        Ok(bytes) => match serde_json::from_slice::<Preferences>(&bytes) {
            Ok(mut prefs) => {
                prefs.timeout_sec = prefs.timeout_sec.clamp(5, 180);
                if prefs.binary_path.trim().is_empty() {
                    prefs.binary_path = default_binary();
                }
                (prefs, None)
            }
            Err(error) => (
                Preferences::default(),
                Some(format!(
                    "설정을 읽지 못했습니다 ({error}). 기존 파일은 보존합니다."
                )),
            ),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => (Preferences::default(), None),
        Err(error) => (
            Preferences::default(),
            Some(format!("설정 읽기 실패: {error}")),
        ),
    }
}

fn atomic_write(file: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    let parent = file
        .parent()
        .ok_or_else(|| io::Error::other("상위 폴더가 없습니다"))?;
    fs::create_dir_all(parent)?;
    let temporary = file.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| {
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        output.write_all(bytes)?;
        output.sync_all()?;
        fs::rename(&temporary, file)?;
        fs::File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn save(prefs: &Preferences) -> Result<(), String> {
    let file = path();
    if file.exists() {
        // Never overwrite a malformed file with defaults.
        let bytes = fs::read(&file).map_err(|e| e.to_string())?;
        serde_json::from_slice::<Preferences>(&bytes).map_err(|e| e.to_string())?;
        let backup = file.with_extension("json.pre-gtk");
        if !backup.exists() {
            atomic_write(&backup, &bytes).map_err(|e| e.to_string())?;
        }
    } else {
        let legacy = PathBuf::from(env::var_os("HOME").unwrap_or_default())
            .join(".config/adguard-gui/config.json");
        if legacy.exists() && legacy != file {
            let bytes = fs::read(legacy).map_err(|e| e.to_string())?;
            atomic_write(&file.with_extension("json.pre-gtk"), &bytes)
                .map_err(|e| e.to_string())?;
        }
    }
    atomic_write(
        &file,
        &serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn sync_autostart(enabled: bool) -> Result<(), String> {
    let file = config_home().join("autostart/adguard-gui.desktop");
    if !enabled {
        return match fs::remove_file(file) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        };
    }
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    let quoted = executable
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    let content = format!("[Desktop Entry]\nType=Application\nName=AdGuard VPN\nExec=\"{quoted}\" --minimized\nIcon=adguard-gui\nTerminal=false\nX-GNOME-Autostart-enabled=true\n");
    atomic_write(&file, content.as_bytes()).map_err(|e| e.to_string())
}

fn default_binary() -> String {
    if let Some(path) = gtk::glib::find_program_in_path("adguardvpn-cli") {
        path.to_string_lossy().into_owned()
    } else if std::path::Path::new("/opt/adguardvpn_cli/adguardvpn-cli").is_file() {
        "/opt/adguardvpn_cli/adguardvpn-cli".into()
    } else {
        "adguardvpn-cli".into()
    }
}
