use fetch_rs::{
    config::{Config, flake_dir},
    util::*,
};
use std::process::Command;

fn main() {
    let path = dirs::config_dir().unwrap_or_default().join("fetch-rs");
    let config_content = Config::read(path.clone()).unwrap_or("".into());
    let cfg = Config::deserialize(config_content);
    let flake_dir = flake_dir(cfg.clone());

    // `spawn()` would probably enable visualizing this
    println!("Rebuilding");
    let rebuild_output = Command::new(format!("{}-rebuild", cfg.rebuild_system))
        .args([cfg.rebuild_cmd.clone(), "--flake".into(), flake_dir])
        .output()
        .expect("Failed to rebuild");
    if rebuild_output.status.success() {
        let rebuild_output_stdout =
            String::from_utf8(rebuild_output.stdout).expect("Unable to stringify rebuild_output");
        println!("  {}", rebuild_output_stdout);
    } else {
        let rebuild_output_stderr =
            String::from_utf8(rebuild_output.stderr).expect("Unable to stringify rebuild_output");
        println!(
            "  Error encountered while rebuilding, doing nothing: {}",
            rebuild_output_stderr
        );
        if cfg.notify {
            notify(cfg);
        }
    }
}
