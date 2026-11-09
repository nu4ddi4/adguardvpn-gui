use gio::prelude::CancellableExt;
use gtk::{gio, glib};
use std::{cell::Cell, rc::Rc, time::Duration};

pub async fn run(binary: &str, args: &[String], timeout: u32) -> Result<String, String> {
    let launcher = gio::SubprocessLauncher::new(
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    );
    launcher.setenv("LANG", "C.UTF-8", true);
    launcher.setenv("LC_ALL", "C.UTF-8", true);
    launcher.setenv("NO_COLOR", "1", true);
    launcher.setenv("LANGUAGE", "en", true);
    let mut command: Vec<&std::ffi::OsStr> = vec![binary.as_ref()];
    command.extend(args.iter().map(|arg| std::ffi::OsStr::new(arg)));
    let child = launcher.spawn(&command).map_err(|error| {
        format!("{binary} 실행 실패: {error}. 공식 CLI 설치 위치를 확인하세요.")
    })?;
    let timed_out = Rc::new(Cell::new(false));
    let cancel = gio::Cancellable::new();
    let deadline_cancel = cancel.clone();
    let marker = timed_out.clone();
    let process = child.clone();
    let timer = glib::timeout_add_local_once(Duration::from_secs(timeout.into()), move || {
        marker.set(true);
        process.force_exit();
        deadline_cancel.cancel();
    });
    let result = gio::GioFuture::new(&child, move |process, _, send| {
        process.communicate_utf8_async(None, Some(&cancel), move |result| send.resolve(result));
    })
    .await;
    if !timed_out.get() {
        timer.remove();
    }
    if timed_out.get() {
        return Err(format!(
            "CLI 응답이 {timeout}초를 초과했습니다. 실제 VPN 상태를 확인하세요."
        ));
    }
    let (stdout, stderr) = result.map_err(|error| error.to_string())?;
    let stdout = stdout.map(|s| s.to_string()).unwrap_or_default();
    let stderr = stderr.map(|s| s.to_string()).unwrap_or_default();
    let output = [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !child.is_successful() {
        return Err(format!(
            "실행 파일: {binary}\n명령: {}\n종료 코드: {}\n{}",
            args.join(" "),
            child.exit_status(),
            crate::model::clean(&output)
        ));
    }
    Ok(output)
}

pub fn login_in_terminal(binary: &str) -> Result<(), String> {
    let binary = glib::find_program_in_path(binary)
        .ok_or_else(|| "VPN CLI를 찾지 못했습니다. 설정의 실행 경로를 확인하세요.".to_owned())?;
    for (terminal, options) in [
        ("xdg-terminal-exec", vec![]),
        ("ptyxis", vec!["--standalone", "--"]),
        ("gnome-terminal", vec!["--wait", "--"]),
        ("konsole", vec!["--separate", "-e"]),
        ("xfce4-terminal", vec!["--disable-server", "-x"]),
        ("xterm", vec!["-e"]),
    ] {
        if let Some(program) = glib::find_program_in_path(terminal) {
            let mut command = std::process::Command::new(program);
            command.args(options).arg(&binary).arg("login");
            match command.spawn() {
                Ok(mut child) => {
                    // 터미널 실행기는 세션을 별도 프로세스로 넘길 수 있습니다.
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                    return Ok(());
                }
                Err(_) => continue,
            }
        }
    }
    Err("터미널을 열지 못했습니다. 직접 adguardvpn-cli login을 실행한 뒤 구독 정보를 새로 고쳐주세요.".into())
}
