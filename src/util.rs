use anyhow::anyhow;
#[cfg(not(target_os = "macos"))]
use std::process::Stdio;
use std::{env, process::Command};

pub enum ExitCode {
    Failure = 1,
    NoOp = 2,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        code as i32
    }
}

/// Retrieve the current system's name from its configuration
#[cfg(not(target_os = "macos"))]
fn system_name() -> Result<String, anyhow::Error> {
    let mut option_proc = Command::new("nixos-option")
        .arg("system.name")
        .stdout(Stdio::piped())
        .spawn()?;
    let option_stdout = option_proc
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Unable to pipe output of nixos-option to sed"))?;

    let mut sed_proc = Command::new("sed")
        .args(["-n", "2p"])
        .stdin(Stdio::from(option_stdout))
        .stdout(Stdio::piped())
        .spawn()?;
    let sed_stdout = sed_proc
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Unable to pipe output of sed to xargs"))?;

    let xargs_output = Command::new("xargs")
        .stdin(Stdio::from(sed_stdout))
        .output()?;

    option_proc.wait()?;
    sed_proc.wait()?;

    String::from_utf8(xargs_output.stdout)
        .map_err(|e| anyhow!("Unable to stringify xargs output: {}", e))
}

/// Retrieve the current system's name from its configuration
#[cfg(target_os = "macos")]
fn system_name() -> Result<String, anyhow::Error> {
    let scutil_output = Command::new("scutil")
        .args(["--get", "LocalHostName"])
        .output()?;

    String::from_utf8(scutil_output.stdout)
        .map_err(|e| anyhow!("Unable to stringify scutil output: {}", e))
}

/// Send a notification via ntfy-sh
pub fn notify() {
    let url = env::var("F_RS_NTFY_URL");
    if url.is_err() {
        tracing::error!("Unable to read $F_RS_NTFY_URL-is it set? Failed to notify");
        return;
    }
    let url = url.unwrap_or_default();
    let message = match system_name() {
        Ok(host) => &format!("Rebuild failed on {}", host),
        _ => "Rebuild failed",
    };
    let mut url_sequence: Vec<&str> = Vec::new();
    if url.contains("https") {
        url_sequence.push("-L");
    }
    url_sequence.push(url.as_ref());
    let mut args = vec!["-d", message];
    args.append(&mut url_sequence);

    tracing::info!("Notifying via ntfy-sh");
    let notify_output = match Command::new("curl").args(args).output() {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Failed to send ntfy notification: {}", e);
            return;
        }
    };
    if notify_output.status.success() {
        let notify_output_stdout = String::from_utf8(notify_output.stdout).unwrap_or_default();
        if !notify_output_stdout.trim().is_empty() {
            tracing::info!("ntfy-sh response: {}", notify_output_stdout.trim());
        } else {
            tracing::info!("ntfy-sh notification sent successfully (no output)");
        }
    } else {
        let notify_output_stderr = String::from_utf8(notify_output.stderr).unwrap_or_default();
        if !notify_output_stderr.trim().is_empty() {
            tracing::error!(
                "ntfy-sh notification failed (stderr): {}",
                notify_output_stderr.trim()
            );
        } else {
            tracing::error!("ntfy-sh notification failed (no stderr output)");
        }
    }
}
