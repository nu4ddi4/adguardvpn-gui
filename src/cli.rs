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
    let mut command: Vec<&std::ffi::OsStr> = vec![binary.as_ref()];
    command.extend(args.iter().map(|arg| std::ffi::OsStr::new(arg)));
    let child = launcher.spawn(&command).map_err(|error| {
        format!("Cannot start {binary}: {error}. Install the official adguardvpn-cli separately and check its path.")
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
            "CLI command timed out after {timeout}s. Check the actual VPN state before retrying."
        ));
    }
    let (stdout, stderr) = result.map_err(|error| error.to_string())?;
    let stdout = stdout.map(|s| s.to_string()).unwrap_or_default();
    let stderr = stderr.map(|s| s.to_string()).unwrap_or_default();
    if !child.is_successful() {
        let message = if stderr.trim().is_empty() {
            &stdout
        } else {
            &stderr
        };
        let text = crate::model::clean(message);
        return Err(if text.is_empty() {
            "CLI failed. Check installation, authentication and the official CLI's required permissions.".into()
        } else {
            text.chars().take(1600).collect()
        });
    }
    Ok(if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    })
}
