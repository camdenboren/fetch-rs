use fetch_rs::{
    config::{CFG_FILE, Config},
    util::*,
};
use std::{path::PathBuf, process::Command};

fn main() {
    let path = PathBuf::from(CFG_FILE);
    let config_content = Config::read(path.clone()).unwrap_or("".into());
    let cfg = Config::deserialize(config_content);

    // `spawn()` would probably enable visualizing this
    println!("Rebuilding");
    let rebuild_output = Command::new(format!("{}-rebuild", cfg.rebuild_system))
        .args([
            cfg.rebuild_cmd.clone(),
            "--flake".into(),
            cfg.flake_dir.clone(),
        ])
        .output()
        .expect("Failed to rebuild");
    if rebuild_output.status.success() {
        let rebuild_output_stdout =
            String::from_utf8(rebuild_output.stdout).expect("Unable to stringify rebuild_output");
        println!("  {}", rebuild_output_stdout);
    } else {
        let rebuild_output_stderr =
            String::from_utf8(rebuild_output.stderr).expect("Unable to stringify rebuild_output");
        eprintln!(
            "  Error encountered while rebuilding, doing nothing: {}",
            rebuild_output_stderr
        );
        if cfg.notify {
            notify(cfg);
        }
    }
}
