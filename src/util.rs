use crate::config::Config;
use std::{io::stdin, process::Command};

/// Prompt the user for input and return it
pub fn user_input(message: &str) -> String {
    println!("{}", message);
    let mut buffer = String::new();
    let stdin = stdin(); // We get `Stdin` here.
    stdin.read_line(&mut buffer).unwrap();
    buffer.trim().into()
}

/// Send a notification via ntfy-sh
pub fn notify(cfg: Config) {
    let url = format!("{}/{}", cfg.ntfy_url, cfg.ntfy_topic);
    let mut url_sequence: Vec<&str> = Vec::new();
    if url.contains("https") {
        url_sequence.push("-L");
    }
    url_sequence.push(url.as_ref());
    let mut args = vec!["-d", "Rebuild failed"];
    args.append(&mut url_sequence);

    println!("Notifying via ntfy-sh server at: {}", url);
    let notify_output = Command::new("curl")
        .args(args)
        .output()
        .expect("Failed to notify");
    if notify_output.status.success() {
        let notify_output_stdout =
            String::from_utf8(notify_output.stdout).expect("Unable to stringify notify_output");
        println!("  {}", notify_output_stdout);
    } else {
        let notify_output_stderr =
            String::from_utf8(notify_output.stderr).expect("Unable to stringify notify_output");
        println!(
            "  Error encountered while rebuilding, doing nothing: {}",
            notify_output_stderr
        );
    }
}
